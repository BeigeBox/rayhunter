PREFIX ?= /usr/local/bin
SCRIPT := $(shell pwd)/usb_ctl.sh
TARGET := $(PREFIX)/usb-ctl

.PHONY: install uninstall help

help:
	@echo "Targets:"
	@echo "  install    Symlink usb-ctl into $(PREFIX)"
	@echo "  uninstall  Remove the symlink"

install: $(SCRIPT)
	@mkdir -p $(PREFIX)
	ln -sf $(SCRIPT) $(TARGET)
	@echo "Installed: $(TARGET) -> $(SCRIPT)"

uninstall:
	rm -f $(TARGET)
	@echo "Removed: $(TARGET)"
