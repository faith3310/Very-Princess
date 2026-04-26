import { webhookRepository } from "../repositories/WebhookRepository.js";
import fetch from "node-fetch";

export class WebhookService {
  async getConfig(organizationId: string) {
    return webhookRepository.getConfig(organizationId);
  }

  async updateConfig(organizationId: string, url: string) {
    return webhookRepository.upsertConfig(organizationId, url);
  }

  async sendTestWebhook(organizationId: string) {
    const config = await webhookRepository.getConfig(organizationId);
    if (!config) throw new Error("Webhook not configured");

    const payload = {
      event: "test",
      timestamp: new Date().toISOString(),
      organizationId,
      message: "This is a test webhook from Very Princess",
    };

    try {
      const response = await fetch(config.url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Webhook-Secret": config.secret,
        },
        body: JSON.stringify(payload),
        timeout: 5000,
      });

      const responseBody = await response.text();
      
      await webhookRepository.createDelivery(
        config.id,
        payload,
        response.status,
        responseBody
      );

      return {
        success: response.ok,
        statusCode: response.status,
        responseBody,
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await webhookRepository.createDelivery(
        config.id,
        payload,
        undefined,
        undefined,
        errorMessage
      );
      throw error;
    }
  }
}

export const webhookService = new WebhookService();
