# Tower Database

Rayhunter can use a local database of known cell towers to detect unknown towers that don't appear in any public registry. This is a strong signal that a tower may be an IMSI catcher.

The database is built from [OpenCellID](https://opencellid.org/), a crowdsourced dataset of cell tower locations. Because the full dataset is too large for a mobile device, Rayhunter ships per-state databases that you select during installation.

## Selecting States During Installation

When running the installer, use the `--tower-states` flag to choose which states to download:

```bash
./installer orbic --admin-password 'mypassword' --tower-states CA,OR,WA
```

You can specify any combination of US state codes. Each state database is typically 1-5 MB:

| State | Towers (LTE+NR) | Size |
|-------|-----------------|------|
| CA | ~66,000 | 10.4 MB |
| TX | ~50,000 | 7.7 MB |
| NY | ~29,000 | 4.5 MB |
| OR | ~6,000 | 0.7 MB |
| WY | ~1,200 | 0.2 MB |

If you travel between states, install databases for all states you expect to visit. You can add more later from the web UI.

## Managing Tower Databases

You can view and manage installed tower databases from the Rayhunter web UI under the **Tower Database** section. This shows:

- Which state databases are currently installed
- Total number of known towers
- Disk space used

### Downloading Additional States

If your device has internet access (via [WiFi client mode](./configuration.md#wifi-client-mode)), you can download additional state databases directly from the web UI:

1. Open the Rayhunter web UI
2. Scroll to the **Tower Database** section
3. Select the states you want to add
4. Click **Download**

The download happens in the background. Each state database is small enough to download over a typical WiFi connection in seconds.

### Removing States

To free up space, you can remove state databases you no longer need from the same section of the web UI. Removing a state database does not affect recordings or analysis of previously captured data.

## Updates

Tower databases are hosted on a dedicated GitHub Release (`tower-db`) that is updated independently of Rayhunter code releases. This means you can get fresh tower data without waiting for a new Rayhunter version.

To update your tower databases, re-download the states you need from the web UI or re-run the installer with `--tower-states`. The installer will replace existing state databases with the latest versions. Your recordings and analysis history are not affected.

## How It Works

When Rayhunter sees a cell tower (via LTE System Information Block messages), it looks up the tower's identity (MCC, MNC, TAC, Cell ID) in the local database. If the tower is not found, Rayhunter flags it as an unknown tower event. This works alongside the existing heuristic analyzers to provide an additional layer of detection.

The tower database uses rectangular bounding boxes for geographic filtering, so a state's database may include some towers just across the border in neighboring states.
