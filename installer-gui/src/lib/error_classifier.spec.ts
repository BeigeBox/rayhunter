import { describe, it, expect } from 'vitest';
import { classify_error } from './error_classifier';

describe('error_classifier', () => {
    describe('connection errors', () => {
        it('matches "Failed to get login info"', () => {
            const result = classify_error('Failed to get login info from 192.168.1.1');
            expect(result.title).toBe("Can't reach the device");
        });

        it('matches "Failed to reach device at"', () => {
            const result = classify_error('Failed to reach device at 192.168.100.1');
            expect(result.title).toBe("Can't reach the device");
        });

        it('matches "Failed to connect to shell"', () => {
            const result = classify_error('Failed to connect to shell on 192.168.0.1:23');
            expect(result.title).toBe("Can't reach the device");
        });

        it('matches top-level install failure', () => {
            const result = classify_error('Failed to install rayhunter on the Orbic RC400L');
            expect(result.title).toBe('Installation failed');
        });
    });

    describe('authentication errors', () => {
        it('matches incorrect password', () => {
            const result = classify_error('Login failed: incorrect password');
            expect(result.title).toBe('Wrong password');
            expect(result.message).toContain('sticker');
        });

        it('matches login retcode failure', () => {
            const result = classify_error('Login failed with retcode: 403');
            expect(result.title).toBe('Login failed');
        });

        it('matches missing token', () => {
            const result = classify_error('login did not return a token in response body');
            expect(result.title).toBe('Login failed');
        });

        it('matches missing password', () => {
            const result = classify_error('--admin-password is required for orbic');
            expect(result.title).toBe('Password required');
        });
    });

    describe('USB errors', () => {
        it('matches no device found', () => {
            const result = classify_error('No Orbic device found on USB');
            expect(result.title).toBe('Device not detected');
        });

        it('matches device busy', () => {
            const result = classify_error('USB device being used by another program');
            expect(result.title).toBe('Device is busy');
        });

        it('matches permission denied', () => {
            const result = classify_error('Permission denied accessing /dev/usb0');
            expect(result.title).toBe('Device is busy');
            expect(result.message).toContain('adb');
        });

        it('matches ADB timeout', () => {
            const result = classify_error('Timeout waiting for ADB connection');
            expect(result.title).toBe('Device not responding');
        });

        it('matches ADB connection error', () => {
            const result = classify_error('ADB connection error: broken pipe');
            expect(result.title).toBe('USB connection error');
        });
    });

    describe('rooting errors', () => {
        it('matches telnet timeout', () => {
            const result = classify_error(
                'Timeout waiting for telnet to become available on port 24'
            );
            expect(result.title).toBe('Rooting took too long');
            expect(result.message).toContain('Power-cycle');
        });

        it('matches rootshell failure', () => {
            const result = classify_error('rootshell is not giving us root access');
            expect(result.title).toBe('Rooting failed');
        });

        it('matches tplink root failure', () => {
            const result = classify_error('Bad result code when trying to root device');
            expect(result.title).toBe('Rooting failed');
        });
    });

    describe('file transfer errors', () => {
        it('matches transfer unsuccessful', () => {
            const result = classify_error('File transfer unsuccessful for rayhunter-daemon');
            expect(result.title).toBe('File transfer corrupted');
        });

        it('matches MD5 failure', () => {
            const result = classify_error('MD5 verification failed for /data/rayhunter/daemon');
            expect(result.title).toBe('File transfer corrupted');
        });

        it('matches command not found', () => {
            const result = classify_error('ls not found in: /usr/bin /bin');
            expect(result.title).toBe('Command failed on device');
        });
    });

    describe('TP-Link SD card errors', () => {
        it('matches no sdcard path', () => {
            const result = classify_error('Unable to determine sdcard path on device');
            expect(result.title).toBe('No SD card found');
        });

        it('matches FAT format error', () => {
            const result = classify_error('Requires a FAT-formatted SD card');
            expect(result.title).toBe('SD card mount failed');
        });
    });

    describe('fallback', () => {
        it('returns generic message for unknown errors', () => {
            const result = classify_error('Something completely unexpected happened');
            expect(result.title).toBe('Something went wrong');
            expect(result.message).toContain('report');
        });

        it('returns generic message for empty string', () => {
            const result = classify_error('');
            expect(result.title).toBe('Something went wrong');
        });
    });
});
