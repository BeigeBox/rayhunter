export type DeviceId =
    | 'orbic'
    | 'orbic-usb'
    | 'moxee'
    | 'tmobile'
    | 'tplink'
    | 'wingtech'
    | 'uz801'
    | 'pinephone';

export type InstallMode = 'install' | 'update';

export interface FieldConfig {
    key: string;
    label: string;
    type: 'text' | 'password' | 'checkbox';
    default_value?: string | boolean;
    arg_name: string;
    advanced?: boolean;
}

export interface StepDefinition {
    label: string;
    marker: string | null;
}

export interface DeviceInfo {
    id: DeviceId;
    label: string;
    command: string;
    fields: FieldConfig[];
    steps: StepDefinition[];
}

export type StepStatus = 'pending' | 'active' | 'done' | 'error';

export interface InstallStep {
    label: string;
    status: StepStatus;
}

export interface ErrorGuidance {
    title: string;
    message: string;
}

export type Overlay = { type: 'tplink_browser'; url: string };

export type Screen =
    | { kind: 'device-select' }
    | { kind: 'config'; device: DeviceInfo }
    | {
          kind: 'progress';
          device: DeviceInfo;
          args: string[];
          mode: InstallMode;
      }
    | { kind: 'success'; admin_ip: string; verified: boolean }
    | {
          kind: 'failure';
          device: DeviceInfo;
          error: string;
          log: string;
          args: string[];
          mode: InstallMode;
      };
