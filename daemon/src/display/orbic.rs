use log::error;

const FB_PATH: &str = "/dev/fb0";
const BL_GPIO_PATH: &str = "/sys/devices/78b6000.spi/spi_master/spi1/spi1.0/bl_gpio";

fn set_backlight(on: bool) {
    let val = if on { "1" } else { "0" };
    if let Err(e) = std::fs::write(BL_GPIO_PATH, val) {
        error!("failed to set backlight via {BL_GPIO_PATH}: {e}");
    }
}

#[cfg(feature = "orbic-ui")]
async fn write_fb_rgb565(rgb888: &[u8]) {
    let mut raw = Vec::with_capacity(rgb888.len() / 3 * 2);
    for chunk in rgb888.chunks_exact(3) {
        let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
        let mut rgb565: u16 = (r as u16 & 0b11111000) << 8;
        rgb565 |= (g as u16 & 0b11111100) << 3;
        rgb565 |= (b as u16) >> 3;
        raw.extend(rgb565.to_le_bytes());
    }
    if let Err(e) = tokio::fs::write(FB_PATH, &raw).await {
        error!("failed to write framebuffer: {e}");
    }
}

// ── orbic-ui: full text-based UI with screen cycling ────────────────

#[cfg(feature = "orbic-ui")]
mod ui {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use embedded_graphics::Pixel;
    use embedded_graphics::framebuffer::Framebuffer;
    use embedded_graphics::mono_font::MonoTextStyle;
    use embedded_graphics::mono_font::ascii::{FONT_4X6, FONT_5X7, FONT_5X8, FONT_6X10};
    use embedded_graphics::pixelcolor::Rgb888;
    use embedded_graphics::pixelcolor::raw::BigEndian;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};
    use embedded_graphics::text::{Alignment, Text};
    use log::{info, warn};
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;
    use tokio::sync::{Notify, RwLock};
    use tokio_util::sync::CancellationToken;
    use tokio_util::task::TaskTracker;

    use crate::display::{DeviceInfo, DisplayState, StoppedReason};
    use rayhunter::analysis::analyzer::EventType;

    const WIDTH: usize = 128;
    const HEIGHT: usize = 128;
    const CX: i32 = WIDTH as i32 / 2;
    const HEADER_HEIGHT: u32 = 52;
    const INPUT_EVENT_SIZE: usize = 32;

    const GREEN: Rgb888 = Rgb888::new(0x00, 0xC8, 0x53);
    const BLUE: Rgb888 = Rgb888::new(0x00, 0x91, 0xEA);
    const YELLOW: Rgb888 = Rgb888::new(0xFF, 0xD6, 0x00);
    const ORANGE: Rgb888 = Rgb888::new(0xFF, 0x6D, 0x00);
    const RED: Rgb888 = Rgb888::new(0xD5, 0x00, 0x00);
    const AMBER: Rgb888 = Rgb888::new(0xFF, 0x8F, 0x00);
    const DARK_RED: Rgb888 = Rgb888::new(0xB7, 0x1C, 0x1C);
    const SEPARATOR: Rgb888 = Rgb888::new(0x33, 0x33, 0x33);
    const LIGHT_GRAY: Rgb888 = Rgb888::new(0xE0, 0xE0, 0xE0);
    const MID_GRAY: Rgb888 = Rgb888::new(0xAA, 0xAA, 0xAA);
    const DIM_GRAY: Rgb888 = Rgb888::new(0x77, 0x77, 0x77);
    const DARK_GRAY: Rgb888 = Rgb888::new(0x66, 0x66, 0x66);
    const DARKER_GRAY: Rgb888 = Rgb888::new(0x44, 0x44, 0x44);

    const BOOT_ORCA_40: &[u8] = include_bytes!("../../images/boot_orca_40.png");
    const BOOT_ORCA_60: &[u8] = include_bytes!("../../images/boot_orca_60.png");
    const BOOT_LOGO_73: &[u8] = include_bytes!("../../images/boot_logo_73.png");
    const BOOT_LOGO_67: &[u8] = include_bytes!("../../images/boot_logo_67.png");

    #[derive(Clone, Copy, PartialEq)]
    enum Screen {
        Status,
        Network,
        System,
        Alerts,
    }

    impl Screen {
        fn next(self) -> Self {
            match self {
                Screen::Status => Screen::Network,
                Screen::Network => Screen::System,
                Screen::System => Screen::Alerts,
                Screen::Alerts => Screen::Status,
            }
        }
    }

    fn accent_color(info: &DeviceInfo) -> Rgb888 {
        if info.colorblind_mode { BLUE } else { GREEN }
    }

    fn state_color(info: &DeviceInfo) -> Rgb888 {
        if info.stopped_reason.is_some() {
            return RED;
        }
        match info.display_state {
            DisplayState::Paused => LIGHT_GRAY,
            DisplayState::Recording => accent_color(info),
            DisplayState::WarningDetected { event_type } => match event_type {
                EventType::Informational => accent_color(info),
                EventType::Low => YELLOW,
                EventType::Medium => ORANGE,
                EventType::High => RED,
            },
        }
    }

    fn state_label(info: &DeviceInfo) -> &'static str {
        if info.stopped_reason.is_some() {
            return "STOPPED";
        }
        match info.display_state {
            DisplayState::Paused => "PAUSED",
            _ => "RECORDING",
        }
    }

    fn draw_text(fb: &mut EgFramebuffer, text: &str, y: i32, font: &MonoTextStyle<Rgb888>) {
        Text::with_alignment(text, Point::new(CX, y), *font, Alignment::Center)
            .draw(fb)
            .ok();
    }

    fn draw_separator(fb: &mut EgFramebuffer, y: i32) {
        Line::new(Point::new(14, y), Point::new(114, y))
            .into_styled(PrimitiveStyle::with_stroke(SEPARATOR, 1))
            .draw(fb)
            .ok();
    }

    fn draw_screen_header(fb: &mut EgFramebuffer, info: &DeviceInfo, title: &str) {
        fb.clear(Rgb888::BLACK).ok();

        let title_style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);
        draw_text(fb, title, 12, &title_style);

        Line::new(Point::new(8, 18), Point::new(120, 18))
            .into_styled(PrimitiveStyle::with_stroke(accent_color(info), 1))
            .draw(fb)
            .ok();
    }

    fn format_uptime(secs: u64) -> String {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        if hours >= 24 {
            format!("{}d {}h", hours / 24, hours % 24)
        } else {
            format!("{}h {:02}m", hours, minutes)
        }
    }

    type EgFramebuffer = Framebuffer<
        Rgb888,
        <Rgb888 as PixelColor>::Raw,
        BigEndian,
        WIDTH,
        HEIGHT,
        { embedded_graphics::framebuffer::buffer_size::<Rgb888>(WIDTH, HEIGHT) },
    >;

    fn blit_png(fb: &mut EgFramebuffer, png_bytes: &[u8], x_offset: i32, y_offset: i32) {
        if let Ok(img) = image::load_from_memory(png_bytes) {
            let rgba = img.to_rgba8();
            for (x, y, pixel) in rgba.enumerate_pixels() {
                if pixel[3] > 0 {
                    let color = Rgb888::new(pixel[0], pixel[1], pixel[2]);
                    Pixel(Point::new(x as i32 + x_offset, y as i32 + y_offset), color)
                        .draw(fb)
                        .ok();
                }
            }
        }
    }

    fn pill_bg_for_block(block: Rgb888) -> Rgb888 {
        match (block.r(), block.g(), block.b()) {
            (0x00, 0xC8, 0x53) => Rgb888::new(0x00, 0x6E, 0x2E),
            (0xFF, 0xD6, 0x00) => Rgb888::new(0x8C, 0x76, 0x00),
            (0xFF, 0x6D, 0x00) => Rgb888::new(0x8C, 0x3C, 0x00),
            (0xD5, 0x00, 0x00) => Rgb888::new(0x75, 0x00, 0x00),
            (0x00, 0x91, 0xEA) => Rgb888::new(0x00, 0x50, 0x81),
            _ => Rgb888::new(0x40, 0x40, 0x40),
        }
    }

    fn draw_banner(fb: &mut EgFramebuffer, bg: Rgb888, text_color: Rgb888, text: &str) {
        Rectangle::new(Point::new(0, 118), Size::new(WIDTH as u32, 10))
            .into_styled(PrimitiveStyle::with_fill(bg))
            .draw(fb)
            .ok();
        let style = MonoTextStyle::new(&FONT_5X7, text_color);
        draw_text(fb, text, 126, &style);
    }

    fn severity_color(event_type: EventType) -> Rgb888 {
        match event_type {
            EventType::High => RED,
            EventType::Medium => ORANGE,
            EventType::Low => YELLOW,
            EventType::Informational => MID_GRAY,
        }
    }

    fn highest_severity_color(counts: &[u32; 4]) -> Rgb888 {
        if counts[EventType::High as usize] > 0 {
            RED
        } else if counts[EventType::Medium as usize] > 0 {
            ORANGE
        } else if counts[EventType::Low as usize] > 0 {
            YELLOW
        } else {
            MID_GRAY
        }
    }

    async fn boot_animation(
        fb: &mut EgFramebuffer,
        device_info: &Arc<RwLock<DeviceInfo>>,
        notify: &Arc<Notify>,
        shutdown_token: &CancellationToken,
    ) {
        let dot_style = MonoTextStyle::new(&FONT_6X10, SEPARATOR);

        // Frame 1: "..."
        fb.clear(Rgb888::BLACK).ok();
        draw_text(fb, "...", 70, &dot_style);
        super::write_fb_rgb565(fb.data()).await;

        tokio::select! {
            _ = shutdown_token.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_millis(500)) => {},
        }

        // Frame 2: small orca
        fb.clear(Rgb888::BLACK).ok();
        blit_png(fb, BOOT_ORCA_40, 44, 44);
        super::write_fb_rgb565(fb.data()).await;

        tokio::select! {
            _ = shutdown_token.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_millis(800)) => {},
        }

        // Frame 3: large orca + logo + version, hold until Recording
        fb.clear(Rgb888::BLACK).ok();
        blit_png(fb, BOOT_ORCA_60, 34, 10);
        blit_png(fb, BOOT_LOGO_67, 30, 78);
        let ver_style = MonoTextStyle::new(&FONT_5X7, DIM_GRAY);
        let ver = format!("v{}", env!("CARGO_PKG_VERSION"));
        draw_text(fb, &ver, 108, &ver_style);
        super::write_fb_rgb565(fb.data()).await;

        // Hold frame 3 for at least 1500ms, then wait for Recording state
        tokio::select! {
            _ = shutdown_token.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_millis(1500)) => {},
        }

        loop {
            {
                let info = device_info.read().await;
                if matches!(info.display_state, DisplayState::Recording) {
                    break;
                }
            }
            tokio::select! {
                _ = shutdown_token.cancelled() => return,
                _ = notify.notified() => {},
                _ = tokio::time::sleep(Duration::from_millis(100)) => {},
            }
        }
    }

    // ── Screen 1: Status ────────────────────────────────────────────

    fn render_status(fb: &mut EgFramebuffer, info: &DeviceInfo) {
        let color = state_color(info);
        fb.clear(Rgb888::BLACK).ok();

        Rectangle::new(Point::zero(), Size::new(WIDTH as u32, HEADER_HEIGHT))
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(fb)
            .ok();

        blit_png(fb, BOOT_LOGO_73, 27, 8);

        let pill_text = if info.stopped_reason.is_some() {
            Some("ERROR")
        } else {
            match info.display_state {
                DisplayState::WarningDetected { event_type } => match event_type {
                    EventType::Low => Some("LOW ALERT"),
                    EventType::Medium => Some("ALERT"),
                    EventType::High => Some("HIGH ALERT"),
                    EventType::Informational => None,
                },
                _ => None,
            }
        };

        if let Some(text) = pill_text {
            let pill_w = text.len() as u32 * 5 + 8;
            let pill_x = (WIDTH as u32 - pill_w) / 2;
            Rectangle::new(Point::new(pill_x as i32, 36), Size::new(pill_w, 12))
                .into_styled(PrimitiveStyle::with_fill(pill_bg_for_block(color)))
                .draw(fb)
                .ok();
            let pill_style = MonoTextStyle::new(&FONT_5X7, Rgb888::WHITE);
            draw_text(fb, text, 45, &pill_style);
        }

        let label_style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);
        draw_text(fb, state_label(info), 66, &label_style);

        draw_separator(fb, 80);

        let disk_color = if info.stopped_reason.is_some()
            && matches!(info.stopped_reason, Some(StoppedReason::DiskFull))
        {
            RED
        } else if info.low_disk {
            AMBER
        } else {
            Rgb888::WHITE
        };

        let disk_text = if matches!(info.stopped_reason, Some(StoppedReason::DiagError)) {
            "check web UI".to_string()
        } else {
            format!("disk: {}MB", info.disk_available_mb)
        };
        let disk_style = MonoTextStyle::new(&FONT_5X7, disk_color);
        draw_text(fb, &disk_text, 92, &disk_style);

        let low_battery =
            matches!(info.battery_level, Some(level) if level < 20) && !info.battery_plugged;
        let batt_color = if low_battery { AMBER } else { Rgb888::WHITE };
        let batt_str = match info.battery_level {
            Some(level) => format!("batt: {}%", level),
            None => "batt: --".to_string(),
        };
        let batt_style = MonoTextStyle::new(&FONT_5X7, batt_color);
        draw_text(fb, &batt_str, 100, &batt_style);

        let banner = match info.stopped_reason {
            Some(StoppedReason::DiskFull) => Some((DARK_RED, Rgb888::WHITE, "DISK FULL")),
            Some(StoppedReason::DiagError) => Some((DARK_RED, Rgb888::WHITE, "DIAG ERROR")),
            None if info.low_disk => Some((AMBER, Rgb888::BLACK, "LOW DISK SPACE")),
            None if low_battery => None,
            _ => None,
        };

        if let Some((bg, text_color, text)) = banner {
            draw_banner(fb, bg, text_color, text);
        } else if low_battery && info.stopped_reason.is_none() && !info.low_disk {
            let text = match info.battery_level {
                Some(level) => format!("LOW BATTERY: {}%", level),
                None => "LOW BATTERY".to_string(),
            };
            draw_banner(fb, AMBER, Rgb888::BLACK, &text);
        } else {
            let total: u32 = info.event_counts.iter().sum();
            let footer_color = if total == 0 {
                DIM_GRAY
            } else {
                highest_severity_color(&info.event_counts)
            };
            let label = if total == 1 { "ALERT" } else { "ALERTS" };
            let footer_style = MonoTextStyle::new(&FONT_4X6, footer_color);
            draw_text(fb, &format!("{total} {label}"), 118, &footer_style);
        }
    }

    // ── Screen 2: Network ───────────────────────────────────────────

    fn render_network(fb: &mut EgFramebuffer, info: &DeviceInfo) {
        draw_screen_header(fb, info, "NETWORK");

        let section_style = MonoTextStyle::new(&FONT_4X6, DARK_GRAY);
        let data_style = MonoTextStyle::new(&FONT_5X8, Rgb888::WHITE);
        let pass_style = MonoTextStyle::new(&FONT_5X8, accent_color(info));
        let ip_style = MonoTextStyle::new(&FONT_5X7, MID_GRAY);

        draw_text(fb, "Access Point", 30, &section_style);

        match info.ap_ssid.as_deref() {
            Some(ssid) => draw_text(fb, ssid, 40, &data_style),
            None => draw_text(fb, "N/A", 40, &data_style),
        }

        match info.ap_password.as_deref() {
            Some(pw) => draw_text(fb, &format!("Pass: {pw}"), 50, &pass_style),
            None => draw_text(fb, "Pass: N/A", 50, &pass_style),
        }

        let addr = format!("192.168.1.1:{}", info.ap_port);
        draw_text(fb, &addr, 60, &ip_style);

        draw_separator(fb, 68);

        draw_text(fb, "WiFi Client", 78, &section_style);

        let dim_style = MonoTextStyle::new(&FONT_5X8, DARK_GRAY);

        if info.wifi_connected {
            if let Some(ref ssid) = info.wifi_ssid {
                draw_text(fb, ssid, 88, &data_style);
            }
            if let Some(ref ip) = info.wifi_ip {
                draw_text(fb, ip, 98, &data_style);
            }
        } else if info.wifi_ssid.is_some() {
            draw_text(fb, "not connected", 88, &dim_style);
        } else {
            draw_text(fb, "not configured", 88, &dim_style);
        }

        draw_separator(fb, 108);

        if info.wifi_connected {
            let footer_style = MonoTextStyle::new(&FONT_4X6, accent_color(info));
            draw_text(fb, "WEB UI AVAILABLE", 118, &footer_style);
        }
    }

    // ── Screen 3: System ────────────────────────────────────────────

    fn render_system(fb: &mut EgFramebuffer, info: &DeviceInfo) {
        draw_screen_header(fb, info, "SYSTEM");

        let data_style = MonoTextStyle::new(&FONT_5X8, Rgb888::WHITE);
        let dim_style = MonoTextStyle::new(&FONT_5X8, MID_GRAY);
        let ver_style = MonoTextStyle::new(&FONT_5X7, MID_GRAY);

        let disk = format!("Disk {}/{}M", info.disk_available_mb, info.disk_total_mb);
        draw_text(fb, &disk, 34, &data_style);

        let mem = format!("Mem  {}/{}M", info.mem_free_mb, info.mem_total_mb);
        draw_text(fb, &mem, 46, &data_style);

        let batt = match info.battery_level {
            Some(level) => format!("Batt    {}%", level),
            None => "Batt    --".to_string(),
        };
        draw_text(fb, &batt, 58, &data_style);

        let up = format!("Up   {}", format_uptime(info.uptime_secs));
        draw_text(fb, &up, 70, &data_style);

        let cell_text = match &info.mcc_mnc {
            Some(plmn) => format!("Cell {plmn}"),
            None => "Cell ---/---".to_string(),
        };
        let cell_style = if info.mcc_mnc.is_some() {
            &data_style
        } else {
            &dim_style
        };
        draw_text(fb, &cell_text, 86, cell_style);

        let rsrp_text = match info.rsrp_dbm {
            Some(dbm) => format!("RSRP {}dBm", dbm),
            None => "RSRP ---".to_string(),
        };
        let rsrp_style = if info.rsrp_dbm.is_some() {
            &data_style
        } else {
            &dim_style
        };
        draw_text(fb, &rsrp_text, 98, rsrp_style);

        let ver = format!("v{}", info.version);
        draw_text(fb, &ver, 118, &ver_style);
    }

    // ── Screen 4: Alerts ────────────────────────────────────────────

    fn render_alerts(fb: &mut EgFramebuffer, info: &DeviceInfo) {
        draw_screen_header(fb, info, "ALERTS");

        let total: u32 = info.event_counts.iter().sum();
        if total == 0 {
            let empty_style = MonoTextStyle::new(&FONT_5X8, DARKER_GRAY);
            draw_text(fb, "No events", 60, &empty_style);
            draw_text(fb, "detected", 72, &empty_style);

            draw_separator(fb, 100);

            let clear_style = MonoTextStyle::new(&FONT_5X8, accent_color(info));
            draw_text(fb, "ALL CLEAR", 114, &clear_style);
            return;
        }

        let counts = &info.event_counts;
        let rows: [(EventType, &str); 4] = [
            (EventType::High, "HIGH"),
            (EventType::Medium, "MEDIUM"),
            (EventType::Low, "LOW"),
            (EventType::Informational, "INFO"),
        ];

        let mut y = 34;
        for (severity, label) in &rows {
            let count = counts[*severity as usize];
            let color = severity_color(*severity);
            let style = MonoTextStyle::new(&FONT_5X8, color);
            draw_text(fb, &format!("{label:>6}: {count:>3}"), y, &style);
            y += 12;
        }

        if let Some(ref time) = info.last_event_time {
            let time_style = MonoTextStyle::new(&FONT_5X7, LIGHT_GRAY);
            draw_text(fb, &format!("Last: {time}"), 86, &time_style);
        }

        if let Some(ref name) = info.last_event_name {
            let name_color = info
                .last_event_severity
                .map(severity_color)
                .unwrap_or(MID_GRAY);
            let name_style = MonoTextStyle::new(&FONT_5X7, name_color);
            if name.len() > 20 {
                draw_text(fb, &name[..20], 100, &name_style);
                let end = name.len().min(40);
                draw_text(fb, &name[20..end], 108, &name_style);
            } else {
                draw_text(fb, name, 100, &name_style);
            }
        }
    }

    // ── Input button reader ─────────────────────────────────────────

    struct InputButton {
        file: Option<File>,
        last_event: Option<Instant>,
    }

    impl InputButton {
        async fn open(path: &str) -> Self {
            let file = match File::open(path).await {
                Ok(f) => Some(f),
                Err(e) => {
                    warn!("{path} unavailable: {e}");
                    None
                }
            };
            Self {
                file,
                last_event: None,
            }
        }

        async fn next_press(&mut self) {
            let file = match self.file.as_mut() {
                Some(f) => f,
                None => {
                    std::future::pending::<()>().await;
                    return;
                }
            };

            let mut buf = [0u8; INPUT_EVENT_SIZE];
            loop {
                if file.read_exact(&mut buf).await.is_err() {
                    std::future::pending::<()>().await;
                    return;
                }

                let now = Instant::now();
                if let Some(last) = self.last_event
                    && now.duration_since(last) < Duration::from_millis(50)
                {
                    self.last_event = Some(now);
                    continue;
                }
                self.last_event = Some(now);

                if buf[12] == 0 {
                    return;
                }
            }
        }
    }

    // ── Display loop ────────────────────────────────────────────────

    const SCREEN_TIMEOUT: Duration = Duration::from_secs(30);

    pub fn update_ui(
        task_tracker: &TaskTracker,
        device_info: Arc<RwLock<DeviceInfo>>,
        notify: Arc<Notify>,
        shutdown_token: CancellationToken,
    ) {
        info!("enabling Orbic backlight via {}", super::BL_GPIO_PATH);
        super::set_backlight(true);

        task_tracker.spawn(async move {
            let mut fb = EgFramebuffer::new();
            let mut wps = InputButton::open("/dev/input/event1").await;
            let mut pwr = InputButton::open("/dev/input/event0").await;

            boot_animation(&mut fb, &device_info, &notify, &shutdown_token).await;

            let mut screen = Screen::Status;
            let mut backlight_on = true;
            let mut last_activity = Instant::now();

            loop {
                if shutdown_token.is_cancelled() {
                    info!("received UI shutdown");
                    break;
                }

                {
                    let info = device_info.read().await;
                    match screen {
                        Screen::Status => render_status(&mut fb, &info),
                        Screen::Network => render_network(&mut fb, &info),
                        Screen::System => render_system(&mut fb, &info),
                        Screen::Alerts => render_alerts(&mut fb, &info),
                    }
                }

                if backlight_on {
                    super::write_fb_rgb565(fb.data()).await;

                    if last_activity.elapsed() >= SCREEN_TIMEOUT {
                        super::set_backlight(false);
                        backlight_on = false;
                    }
                }

                tokio::select! {
                    _ = shutdown_token.cancelled() => break,
                    _ = notify.notified() => {
                        let mut info = device_info.write().await;
                        if info.wake_display {
                            info.wake_display = false;
                            drop(info);
                            if !backlight_on {
                                super::set_backlight(true);
                                backlight_on = true;
                                screen = Screen::Status;
                            }
                            last_activity = Instant::now();
                        }
                    },
                    _ = wps.next_press() => {
                        if backlight_on {
                            screen = screen.next();
                        } else {
                            super::set_backlight(true);
                            backlight_on = true;
                        }
                        last_activity = Instant::now();
                    },
                    _ = pwr.next_press() => {
                        if backlight_on {
                            screen = screen.next();
                        } else {
                            super::set_backlight(true);
                            backlight_on = true;
                        }
                        last_activity = Instant::now();
                    },
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {},
                }
            }
        });
    }
}

// ── fallback: existing GenericFramebuffer colored-line path ───────────

#[cfg(not(feature = "orbic-ui"))]
mod fallback {
    use crate::config;
    use crate::display::DisplayState;
    use crate::display::generic_framebuffer::{self, Dimensions, GenericFramebuffer};
    use async_trait::async_trait;
    use log::info;
    use tokio::sync::mpsc::Receiver;
    use tokio_util::sync::CancellationToken;
    use tokio_util::task::TaskTracker;

    #[derive(Copy, Clone, Default)]
    struct LegacyFramebuffer;

    #[async_trait]
    impl GenericFramebuffer for LegacyFramebuffer {
        fn dimensions(&self) -> Dimensions {
            Dimensions {
                height: 128,
                width: 128,
            }
        }

        async fn write_buffer(&mut self, buffer: Vec<(u8, u8, u8)>) {
            let mut raw_buffer = Vec::new();
            for (r, g, b) in buffer {
                let mut rgb565: u16 = (r as u16 & 0b11111000) << 8;
                rgb565 |= (g as u16 & 0b11111100) << 3;
                rgb565 |= (b as u16) >> 3;
                raw_buffer.extend(rgb565.to_le_bytes());
            }
            tokio::fs::write(super::FB_PATH, &raw_buffer).await.unwrap();
        }
    }

    pub fn update_ui(
        task_tracker: &TaskTracker,
        config: &config::Config,
        shutdown_token: CancellationToken,
        ui_update_rx: Receiver<DisplayState>,
    ) {
        info!("enabling Orbic backlight via {}", super::BL_GPIO_PATH);
        super::set_backlight(true);
        generic_framebuffer::update_ui(
            task_tracker,
            config,
            LegacyFramebuffer,
            shutdown_token,
            ui_update_rx,
        )
    }
}

// ── public API: dispatch based on feature ────────────────────────────

#[cfg(feature = "orbic-ui")]
pub use ui::update_ui;

#[cfg(not(feature = "orbic-ui"))]
pub use fallback::update_ui;
