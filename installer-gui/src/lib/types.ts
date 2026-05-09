export type DeviceId =
    | 'orbic'
    | 'orbic-usb'
    | 'tmobile'
    | 'tplink'
    | 'wingtech'
    | 'uz801'
    | 'pinephone';

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

export interface InstallStep {
    label: string;
    status: 'pending' | 'active' | 'done' | 'error';
}

export interface ErrorGuidance {
    title: string;
    message: string;
    context?: string[];
}

export type Screen =
    | { kind: 'device-select' }
    | { kind: 'config'; device: DeviceInfo }
    | { kind: 'progress'; device: DeviceInfo; args: string; mode: 'install' | 'update' }
    | {
          kind: 'success';
          device: DeviceInfo;
          admin_ip: string;
          verified: boolean;
      }
    | {
          kind: 'failure';
          device: DeviceInfo;
          error: ErrorGuidance;
          log: string;
          args: string;
      };
