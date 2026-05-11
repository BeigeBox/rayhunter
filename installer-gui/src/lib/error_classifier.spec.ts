import { describe, it, expect } from 'vitest';
import { classify_error } from './error_classifier';

describe('error_classifier', () => {
    it('matches a known pattern (incorrect password)', () => {
        const result = classify_error('Login failed: incorrect password');
        expect(result.title).toBe('Wrong password');
        expect(result.message).not.toContain('sticker');
    });

    it('falls back to a generic message that points to Copy Log / Report Issue', () => {
        const result = classify_error('Something completely unexpected happened');
        expect(result.title).toBe('Something went wrong');
        expect(result.message).toMatch(/Copy Log/);
        expect(result.message).toMatch(/Report Issue/);
    });

    it('classifies cancellation distinctly from a bug', () => {
        const result = classify_error(
            'Failed to install rayhunter on the Orbic RC400L\n\nCaused by:\n    Installation cancelled.'
        );
        expect(result.title).toBe('Installation cancelled');
        expect(result.message).not.toMatch(/Report Issue/i);
    });

    it('only matches "Permission denied" in a USB context', () => {
        const fs = classify_error('Permission denied opening /var/log/install.log');
        expect(fs.title).toBe('Something went wrong');

        const usb = classify_error(
            'libusb error: Permission denied (LIBUSB_ERROR_ACCESS) opening device'
        );
        expect(usb.title).toBe('USB device is busy');
    });

    it('masks --admin-password before matching to avoid leaking secrets', () => {
        const result = classify_error(
            "error: unexpected argument 'extra' --admin-password hunter2 --admin-ip 1.2.3.4"
        );
        expect(JSON.stringify(result)).not.toContain('hunter2');
    });
});
