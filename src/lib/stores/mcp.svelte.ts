/// State Kontrol AI (MCP): nyalakan/matikan server dan tampilkan endpoint + token.

import { api, errorMessage, type McpStatus } from "../api";
import { app } from "./app.svelte";

class McpStore {
  running = $state(false);
  port = $state(8787);
  endpoint = $state<string | null>(null);
  token = $state<string | null>(null);
  busy = $state(false);

  private apply(status: McpStatus) {
    this.running = status.running;
    this.port = status.port;
    this.endpoint = status.endpoint;
    this.token = status.token;
  }

  async refresh() {
    try {
      this.apply(await api.mcpStatus());
    } catch {
      /* status tak terbaca: biarkan seperti sekarang */
    }
  }

  async start(port?: number) {
    this.busy = true;
    try {
      this.apply(await api.mcpStart(port ?? this.port));
      app.showToast("Kontrol AI diaktifkan", "success");
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    } finally {
      this.busy = false;
    }
  }

  async stop() {
    this.busy = true;
    try {
      this.apply(await api.mcpStop());
    } catch (error) {
      app.showToast(errorMessage(error), "danger");
    } finally {
      this.busy = false;
    }
  }
}

export const mcp = new McpStore();
