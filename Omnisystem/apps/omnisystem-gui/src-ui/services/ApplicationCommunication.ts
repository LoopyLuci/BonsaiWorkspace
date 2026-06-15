/**
 * INTER-APPLICATION COMMUNICATION SERVICE
 * Enables messaging and service calls between applications
 */

import {
  AppMessage,
  AppMessageHandler,
  AppService,
  ServiceRequest,
  ServiceResponse,
  ApplicationError,
} from '../types/ApplicationTypes';

export class ApplicationCommunication {
  private messageQueue: Map<string, AppMessage[]> = new Map();
  private messageHandlers: Map<string, AppMessageHandler[]> = new Map();
  private services: Map<string, AppService> = new Map();
  private requestIdCounter = 0;

  // =========================================================================
  // MESSAGE OPERATIONS
  // =========================================================================

  /**
   * Send a message to another application
   */
  async sendMessage(message: AppMessage): Promise<string> {
    try {
      // Validate message
      this.validateMessage(message);

      // Add message ID if not present
      if (!message.id) {
        message.id = `msg_${++this.requestIdCounter}`;
      }

      // Set timestamp
      message.timestamp = new Date();

      // Queue the message
      if (!this.messageQueue.has(message.to)) {
        this.messageQueue.set(message.to, []);
      }

      const queue = this.messageQueue.get(message.to)!;
      queue.push(message);

      // Process message immediately if handlers exist
      await this.processMessage(message);

      console.log(`📨 Sent message from ${message.from} to ${message.to}: ${message.type}`);

      return message.id;
    } catch (error) {
      throw new ApplicationError(
        `Failed to send message: ${String(error)}`
      );
    }
  }

  /**
   * Broadcast a message to all applications
   */
  async broadcast(message: Omit<AppMessage, 'to'>): Promise<string[]> {
    const messageIds: string[] = [];

    try {
      // Create a broadcast message for each registered handler
      const broadcastMessage: AppMessage = {
        ...(message as AppMessage),
        to: 'broadcast',
        id: `broadcast_${++this.requestIdCounter}`,
      };

      // Get all unique application IDs from handlers
      const appIds = new Set<string>();
      for (const appId of this.messageHandlers.keys()) {
        appIds.add(appId);
      }

      // Send to each application
      for (const appId of appIds) {
        const targetMessage = { ...broadcastMessage, to: appId };
        const msgId = await this.sendMessage(targetMessage);
        messageIds.push(msgId);
      }

      console.log(
        `📡 Broadcasted message to ${appIds.size} applications`
      );

      return messageIds;
    } catch (error) {
      throw new ApplicationError(
        `Failed to broadcast message: ${String(error)}`
      );
    }
  }

  /**
   * Process queued messages for an application
   */
  async processMessages(appId: string): Promise<void> {
    const queue = this.messageQueue.get(appId);
    if (!queue || queue.length === 0) {
      return;
    }

    console.log(`Processing ${queue.length} messages for ${appId}`);

    while (queue.length > 0) {
      const message = queue.shift()!;
      await this.processMessage(message);
    }
  }

  /**
   * Process a single message
   */
  private async processMessage(message: AppMessage): Promise<void> {
    try {
      const handlers = this.messageHandlers.get(message.to) || [];

      for (const handler of handlers) {
        if (handler.messageType === message.type || handler.messageType === '*') {
          try {
            await handler.handler(message);
          } catch (error) {
            console.error(
              `Handler error for ${message.type}:`,
              error
            );
          }
        }
      }
    } catch (error) {
      console.error('Message processing error:', error);
    }
  }

  // =========================================================================
  // MESSAGE HANDLER MANAGEMENT
  // =========================================================================

  /**
   * Register a message handler
   */
  registerHandler(
    appId: string,
    messageType: string,
    handler: (message: AppMessage) => Promise<void>
  ): () => void {
    if (!this.messageHandlers.has(appId)) {
      this.messageHandlers.set(appId, []);
    }

    const handlers = this.messageHandlers.get(appId)!;
    const handlerEntry: AppMessageHandler = {
      messageType,
      handler,
    };

    handlers.push(handlerEntry);

    console.log(`📌 Registered handler for ${appId}.${messageType}`);

    // Return unsubscribe function
    return () => {
      const index = handlers.indexOf(handlerEntry);
      if (index > -1) {
        handlers.splice(index, 1);
        console.log(`📌 Unregistered handler for ${appId}.${messageType}`);
      }
    };
  }

  /**
   * Get message queue for an application
   */
  getMessageQueue(appId: string): AppMessage[] {
    return this.messageQueue.get(appId) || [];
  }

  /**
   * Clear message queue
   */
  clearQueue(appId: string): void {
    this.messageQueue.delete(appId);
  }

  // =========================================================================
  // SERVICE MANAGEMENT
  // =========================================================================

  /**
   * Register an application service
   */
  registerService(service: AppService): void {
    try {
      if (this.services.has(service.id)) {
        throw new ApplicationError(
          `Service ${service.id} already registered`
        );
      }

      this.services.set(service.id, service);
      console.log(`🔧 Registered service: ${service.name}`);
    } catch (error) {
      throw new ApplicationError(
        `Failed to register service: ${String(error)}`
      );
    }
  }

  /**
   * Unregister a service
   */
  unregisterService(serviceId: string): void {
    if (this.services.delete(serviceId)) {
      console.log(`🔧 Unregistered service: ${serviceId}`);
    }
  }

  /**
   * Get a registered service
   */
  getService(serviceId: string): AppService | null {
    return this.services.get(serviceId) || null;
  }

  /**
   * List services for an application
   */
  getServicesForApp(appId: string): AppService[] {
    return Array.from(this.services.values()).filter(
      (s) => s.appId === appId
    );
  }

  /**
   * Call a remote service method
   */
  async callService(request: ServiceRequest): Promise<ServiceResponse> {
    try {
      const service = this.services.get(request.serviceId);
      if (!service) {
        throw new ApplicationError(
          `Service ${request.serviceId} not found`
        );
      }

      if (!service.methods.includes(request.method)) {
        throw new ApplicationError(
          `Method ${request.method} not found in service`
        );
      }

      // Create a service message
      const message: AppMessage = {
        id: `service_${++this.requestIdCounter}`,
        from: request.fromApp,
        to: service.appId,
        type: `service:${request.serviceId}:${request.method}`,
        data: request.params,
        timestamp: new Date(),
        priority: 'normal',
        requiresAck: true,
      };

      // Send the service request
      await this.sendMessage(message);

      // In production, wait for response with timeout
      return {
        requestId: message.id,
        success: true,
        result: { status: 'pending' },
      };
    } catch (error) {
      return {
        requestId: '',
        success: false,
        error: String(error),
      };
    }
  }

  // =========================================================================
  // VALIDATION
  // =========================================================================

  /**
   * Validate message structure
   */
  private validateMessage(message: AppMessage): void {
    const errors: string[] = [];

    if (!message.from) errors.push('Message.from is required');
    if (!message.to) errors.push('Message.to is required');
    if (!message.type) errors.push('Message.type is required');

    if (errors.length > 0) {
      throw new ApplicationError(`Message validation failed: ${errors.join(', ')}`);
    }
  }

  // =========================================================================
  // STATISTICS
  // =========================================================================

  /**
   * Get communication statistics
   */
  getStatistics() {
    const queueStats: Record<string, number> = {};
    let totalQueued = 0;

    for (const [appId, queue] of Array.from(this.messageQueue.entries())) {
      queueStats[appId] = queue.length;
      totalQueued += queue.length;
    }

    return {
      totalQueued,
      queuedByApp: queueStats,
      registeredHandlers: this.messageHandlers.size,
      registeredServices: this.services.size,
    };
  }

  /**
   * Clear all messages and services (for testing)
   */
  clear(): void {
    this.messageQueue.clear();
    this.messageHandlers.clear();
    this.services.clear();
    console.log('🧹 Cleared all messages and services');
  }
}

// Create singleton instance
export const applicationCommunication = new ApplicationCommunication();
