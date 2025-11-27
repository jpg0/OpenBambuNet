import { EventEmitter } from 'events';

export interface PrinterConfig {
  devId: string;
  devName: string;
  model: string;
  password: string;
  ip: string;
}

export interface ReceivedMessage {
  topic: string;
  payload: string;
  timestamp: Date;
}

export interface UploadedFile {
  filename: string;
  size: number;
  timestamp: Date;
  content: Buffer;
}

/**
 * Manages the state of the mock printer, including configuration,
 * received messages, uploaded files, and connection status.
 */
export class PrinterState extends EventEmitter {
  private config: PrinterConfig;
  private receivedMessages: ReceivedMessage[] = [];
  private uploadedFiles: UploadedFile[] = [];
  private connected: boolean = false;
  private autoRespond: boolean = true;

  constructor(config: PrinterConfig) {
    super();
    this.config = config;
  }

  // Configuration
  getConfig(): PrinterConfig {
    return { ...this.config };
  }

  updateConfig(updates: Partial<PrinterConfig>): void {
    this.config = { ...this.config, ...updates };
    this.emit('config-updated', this.config);
  }

  // Connection status
  setConnected(connected: boolean): void {
    this.connected = connected;
    this.emit('connection-changed', connected);
  }

  isConnected(): boolean {
    return this.connected;
  }

  // Message tracking
  addReceivedMessage(topic: string, payload: string): void {
    const message: ReceivedMessage = {
      topic,
      payload,
      timestamp: new Date(),
    };
    this.receivedMessages.push(message);
    this.emit('message-received', message);
    console.log(`[State] Message received on ${topic}: ${payload.substring(0, 100)}...`);
  }

  getReceivedMessages(): ReceivedMessage[] {
    return [...this.receivedMessages];
  }

  clearReceivedMessages(): void {
    this.receivedMessages = [];
  }

  // File tracking
  addUploadedFile(filename: string, content: Buffer): void {
    const file: UploadedFile = {
      filename,
      size: content.length,
      timestamp: new Date(),
      content,
    };
    this.uploadedFiles.push(file);
    this.emit('file-uploaded', file);
    console.log(`[State] File uploaded: ${filename} (${content.length} bytes)`);
  }

  getUploadedFiles(): UploadedFile[] {
    return this.uploadedFiles.map(f => ({
      filename: f.filename,
      size: f.size,
      timestamp: f.timestamp,
      content: f.content,
    }));
  }

  getUploadedFile(filename: string): UploadedFile | undefined {
    return this.uploadedFiles.find(f => f.filename === filename);
  }

  clearUploadedFiles(): void {
    this.uploadedFiles = [];
  }

  // Auto-respond configuration
  setAutoRespond(enabled: boolean): void {
    this.autoRespond = enabled;
  }

  shouldAutoRespond(): boolean {
    return this.autoRespond;
  }

  // Generate a realistic printer status message
  generateStatusMessage(): string {
    return JSON.stringify({
      print: {
        gcode_state: 'IDLE',
        mc_percent: 0,
        mc_remaining_time: 0,
        s_obj: [],
        layer_num: 0,
        total_layer_num: 0,
      },
      info: {
        dev_id: this.config.devId,
        dev_model_name: this.config.model,
        name: this.config.devName,
      },
      system: {
        command: 'push_status',
        sequence_id: Date.now().toString(),
      },
    });
  }

  // Reset all state
  reset(): void {
    this.receivedMessages = [];
    this.uploadedFiles = [];
    this.connected = false;
    this.autoRespond = true;
    this.emit('reset');
    console.log('[State] State reset');
  }
}
