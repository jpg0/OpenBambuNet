import { FtpSrv } from 'ftp-srv';
import { Readable, Writable } from 'stream';
import { PrinterState } from './printer-state';
import * as path from 'path';

export interface FtpServerConfig {
  port: number;
  useTls: boolean;
  certPath?: string;
  keyPath?: string;
}

/**
 * Custom file system for the FTP server that stores files in memory
 * and tracks them in the printer state.
 */
function createMemoryFileSystem(state: PrinterState): any {
  return {
    currentDirectory: () => '/',
    
    get: (filename: string) => {
      const basename = path.basename(filename);
      return Promise.resolve({
        name: basename,
        isDirectory: () => false,
        size: 0,
        mtime: new Date(),
      });
    },

    list: async (dirname: string = '/') => {
      const files = state.getUploadedFiles();
      return files.map(f => ({
        name: f.filename,
        isDirectory: () => false,
        size: f.size,
        mtime: f.timestamp,
      }));
    },

    chdir: async (newPath: string) => {
      return newPath;
    },

    write: (filename: string, { append = false, start = 0 } = {}) => {
      const chunks: Buffer[] = [];
      const writable = new Writable({
        write(chunk: any, encoding: any, callback: any) {
          chunks.push(chunk);
          callback();
        }
      });

      writable.on('finish', () => {
        const content = Buffer.concat(chunks);
        const basename = path.basename(filename);
        state.addUploadedFile(basename, content);
      });

      return Promise.resolve(writable);
    },

    read: async (filename: string, { start = 0 } = {}) => {
      const basename = path.basename(filename);
      const file = state.getUploadedFile(basename);
      
      if (!file) {
        return new Readable({
          read() {
            this.push(null);
          },
        });
      }

      return new Readable({
        read() {
          this.push(file.content);
          this.push(null);
        },
      });
    },

    delete: async (filename: string) => {
      console.log(`[FTP] Delete requested for: ${filename}`);
    },

    mkdir: async (dirname: string) => {
      console.log(`[FTP] Mkdir requested for: ${dirname}`);
      return dirname;
    },

    rename: async (from: string, to: string) => {
      console.log(`[FTP] Rename requested from ${from} to ${to}`);
      return to;
    },

    chmod: async (filename: string, mode: string) => {
      console.log(`[FTP] Chmod requested for ${filename}: ${mode}`);
    },

    getUniqueName: () => {
      return `file_${Date.now()}`;
    },
  };
}

/**
 * FTP server that simulates a Bambu Lab printer's FTP interface.
 * Handles file uploads and stores them in the printer state.
 */
export class FtpServer {
  private ftpServer: FtpSrv | null = null;
  private state: PrinterState;
  private config: FtpServerConfig;

  constructor(state: PrinterState, config: FtpServerConfig) {
    this.state = state;
    this.config = config;
  }

  /**
   * Start the FTP server
   */
  async start(): Promise<void> {
    const protocol = this.config.useTls ? 'ftps' : 'ftp';
    const url = `${protocol}://0.0.0.0:${this.config.port}`;

    const ftpConfig: any = {
      url,
      pasv_url: '127.0.0.1',
      pasv_min: 50000,
      pasv_max: 51000,
      greeting: 'Bambu Lab Mock Printer FTP',
      anonymous: false,
    };

    if (this.config.useTls && this.config.certPath && this.config.keyPath) {
      ftpConfig.tls = {
        key: this.config.keyPath,
        cert: this.config.certPath,
      };
    }

    this.ftpServer = new FtpSrv(ftpConfig);

    // Authentication
    this.ftpServer.on('login', ({ connection, username, password }, resolve, reject) => {
      const printerConfig = this.state.getConfig();
      
      if (username === 'bblp' && password === printerConfig.password) {
        console.log(`[FTP] User authenticated: ${username}`);
        resolve({ fs: createMemoryFileSystem(this.state) });
      } else {
        console.log(`[FTP] Authentication failed for: ${username}`);
        reject(new Error('Invalid credentials'));
      }
    });

    // Client connection
    this.ftpServer.on('client-error', ({ connection, context, error }) => {
      console.error('[FTP] Client error:', error.message);
    });

    return new Promise((resolve, reject) => {
      this.ftpServer!.listen()
        .then(() => {
          console.log(`[FTP] Server listening on ${url}`);
          resolve();
        })
        .catch((err: Error) => {
          console.error('[FTP] Failed to start:', err);
          reject(err);
        });
    });
  }

  /**
   * Stop the FTP server
   */
  async stop(): Promise<void> {
    if (this.ftpServer) {
      await this.ftpServer.close();
      console.log('[FTP] Server stopped');
    }
  }
}
