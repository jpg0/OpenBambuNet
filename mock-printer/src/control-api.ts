import express, { Express, Request, Response } from 'express';
import { Server } from 'http';
import { PrinterState, PrinterConfig } from './printer-state';
import { MqttBroker } from './mqtt-broker';

export interface ControlApiConfig {
  port: number;
}

/**
 * REST API for controlling the mock printer during tests.
 * Allows tests to inject messages, verify state, and configure behavior.
 */
export class ControlApi {
  private app: Express;
  private server: Server | null = null;
  private state: PrinterState;
  private mqttBroker: MqttBroker;
  private config: ControlApiConfig;

  constructor(state: PrinterState, mqttBroker: MqttBroker, config: ControlApiConfig) {
    this.state = state;
    this.mqttBroker = mqttBroker;
    this.config = config;
    this.app = express();
    
    this.app.use(express.json());
    this.setupRoutes();
  }

  private setupRoutes(): void {
    // Health check
    this.app.get('/health', (req: Request, res: Response) => {
      res.json({
        status: 'ok',
        connected: this.state.isConnected(),
        timestamp: new Date().toISOString(),
      });
    });

    // Get printer configuration
    this.app.get('/config', (req: Request, res: Response) => {
      res.json(this.state.getConfig());
    });

    // Update printer configuration
    this.app.post('/config', (req: Request, res: Response) => {
      const updates: Partial<PrinterConfig> = req.body;
      this.state.updateConfig(updates);
      res.json({
        success: true,
        config: this.state.getConfig(),
      });
    });

    // Get received messages
    this.app.get('/messages/received', (req: Request, res: Response) => {
      res.json({
        messages: this.state.getReceivedMessages(),
        count: this.state.getReceivedMessages().length,
      });
    });

    // Inject a message (publish to report topic)
    this.app.post('/messages', (req: Request, res: Response) => {
      const { message } = req.body;
      
      if (!message) {
        res.status(400).json({ error: 'Message is required' });
        return;
      }

      try {
        this.mqttBroker.publishMessage(message);
        res.json({ success: true });
      } catch (error) {
        res.status(500).json({
          error: 'Failed to publish message',
          details: error instanceof Error ? error.message : 'Unknown error',
        });
      }
    });

    // Publish status message
    this.app.post('/messages/status', (req: Request, res: Response) => {
      try {
        this.mqttBroker.publishStatus();
        res.json({ success: true });
      } catch (error) {
        res.status(500).json({
          error: 'Failed to publish status',
          details: error instanceof Error ? error.message : 'Unknown error',
        });
      }
    });

    // Clear received messages
    this.app.delete('/messages/received', (req: Request, res: Response) => {
      this.state.clearReceivedMessages();
      res.json({ success: true });
    });

    // Get uploaded files
    this.app.get('/files', (req: Request, res: Response) => {
      const files = this.state.getUploadedFiles().map(f => ({
        filename: f.filename,
        size: f.size,
        timestamp: f.timestamp,
      }));
      
      res.json({
        files,
        count: files.length,
      });
    });

    // Get specific file content
    this.app.get('/files/:filename', (req: Request, res: Response) => {
      const file = this.state.getUploadedFile(req.params.filename);
      
      if (!file) {
        res.status(404).json({ error: 'File not found' });
        return;
      }

      res.json({
        filename: file.filename,
        size: file.size,
        timestamp: file.timestamp,
        content: file.content.toString('base64'),
      });
    });

    // Clear uploaded files
    this.app.delete('/files', (req: Request, res: Response) => {
      this.state.clearUploadedFiles();
      res.json({ success: true });
    });

    // Configure auto-respond
    this.app.post('/auto-respond', (req: Request, res: Response) => {
      const { enabled } = req.body;
      
      if (typeof enabled !== 'boolean') {
        res.status(400).json({ error: 'enabled must be a boolean' });
        return;
      }

      this.state.setAutoRespond(enabled);
      res.json({ success: true, enabled });
    });

    // Reset all state
    this.app.post('/reset', (req: Request, res: Response) => {
      this.state.reset();
      res.json({ success: true });
    });
  }

  /**
   * Start the control API server
   */
  async start(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.server = this.app.listen(this.config.port, () => {
          console.log(`[Control API] Listening on http://0.0.0.0:${this.config.port}`);
          resolve();
        });

        this.server.on('error', (err) => {
          console.error('[Control API] Server error:', err);
          reject(err);
        });
      } catch (err) {
        reject(err);
      }
    });
  }

  /**
   * Stop the control API server
   */
  async stop(): Promise<void> {
    return new Promise((resolve) => {
      if (this.server) {
        this.server.close(() => {
          console.log('[Control API] Server stopped');
          resolve();
        });
      } else {
        resolve();
      }
    });
  }
}
