export type Severity = 'error' | 'warning' | 'info' | 'hint';

export interface Diagnostic {
  rule_id: string;
  file: string;
  line: number;
  column: number;
  severity: string;
  message: string;
  fix?: string;
}

export * from './inference_mode';
export * from './model_data';
