import { PrinterState, PrinterConfig } from './printer-state';
import { MqttBroker, MqttBrokerConfig } from './mqtt-broker';
import { FtpServer, FtpServerConfig } from './ftp-server';
import { ControlApi, ControlApiConfig } from './control-api';

export interface MockPrinterOptions {
  devId?: string;
  devName?: string;
  model?: string;
  password?: string;
  ip?: string;
  mqttPort?: number;
  ftpPort?: number;
  controlPort?: number;
  useTls?: boolean;
  certPath?: string;
  keyPath?: string;
}

/**
 * Main entry point for the mock printer.
 * Orchestrates MQTT broker, FTP server, and control API.
 */
export class MockPrinter {
  private state: PrinterState;
  private mqttBroker: MqttBroker;
  private ftpServer: FtpServer;
  private controlApi: ControlApi;

  constructor(options: MockPrinterOptions = {}) {
    // Default configuration
    const printerConfig: PrinterConfig = {
      devId: options.devId || '01234567',
      devName: options.devName || 'Mock Printer',
      model: options.model || '3DPrinter-X1-Carbon',
      password: options.password || 'testpass',
      ip: options.ip || '127.0.0.1',
    };

    this.state = new PrinterState(printerConfig);

    const mqttConfig: MqttBrokerConfig = {
      port: options.mqttPort || 8883,
      useTls: options.useTls || false,
      certPath: options.certPath,
      keyPath: options.keyPath,
    };

    const ftpConfig: FtpServerConfig = {
      port: options.ftpPort || 2121,
      useTls: options.useTls || false,
      certPath: options.certPath,
      keyPath: options.keyPath,
    };

    const controlConfig: ControlApiConfig = {
      port: options.controlPort || 3000,
    };

    this.mqttBroker = new MqttBroker(this.state, mqttConfig);
    this.ftpServer = new FtpServer(this.state, ftpConfig);
    this.controlApi = new ControlApi(this.state, this.mqttBroker, controlConfig);
  }

  /**
   * Start all services
   */
  async start(): Promise<void> {
    console.log('='.repeat(60));
    console.log('Starting Bambu Mock Printer');
    console.log('='.repeat(60));
    console.log('Configuration:', this.state.getConfig());
    console.log('='.repeat(60));

    try {
      await this.mqttBroker.start();
      await this.ftpServer.start();
      await this.controlApi.start();
      
      console.log('='.repeat(60));
      console.log('Mock Printer Ready');
      console.log('='.repeat(60));
    } catch (error) {
      console.error('Failed to start mock printer:', error);
      throw error;
    }
  }

  /**
   * Stop all services
   */
  async stop(): Promise<void> {
    console.log('Stopping mock printer...');
    
    await this.controlApi.stop();
    await this.ftpServer.stop();
    await this.mqttBroker.stop();
    
    console.log('Mock printer stopped');
  }
}

// CLI entry point
async function main() {
  const options: MockPrinterOptions = {
    devId: process.env.DEV_ID || '01234567',
    devName: process.env.DEV_NAME || 'Mock Printer',
    model: process.env.MODEL || '3DPrinter-X1-Carbon',
    password: process.env.PASSWORD || 'testpass',
    ip: process.env.IP || '127.0.0.1',
    mqttPort: parseInt(process.env.MQTT_PORT || '8883'),
    ftpPort: parseInt(process.env.FTP_PORT || '2121'),
    controlPort: parseInt(process.env.CONTROL_PORT || '3000'),
    useTls: process.env.USE_TLS === 'true',
    certPath: process.env.CERT_PATH,
    keyPath: process.env.KEY_PATH,
  };

  const printer = new MockPrinter(options);

  // Handle shutdown gracefully
  process.on('SIGINT', async () => {
    console.log('\nReceived SIGINT, shutting down...');
    await printer.stop();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    console.log('\nReceived SIGTERM, shutting down...');
    await printer.stop();
    process.exit(0);
  });

  try {
    await printer.start();
  } catch (error) {
    console.error('Failed to start:', error);
    process.exit(1);
  }
}

// Run if executed directly
if (require.main === module) {
  main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
  });
}

export default MockPrinter;
