"use client";

import { useEffect, useState } from "react";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../components/ui/tabs";
import {
  detectIntegrations,
  getAvailableIntegrations,
  listIntegrations,
} from "../lib/tauri-api";
import type { Integration } from "../types/integration";
import { toast } from "@/lib/toast";

function parseCapabilities(metadataJson: string | null): {
  itemTypes: string[];
} {
  if (!metadataJson) {
    return { itemTypes: [] };
  }

  try {
    const parsed = JSON.parse(metadataJson) as {
      item_types?: string[];
    };
    return {
      itemTypes: Array.isArray(parsed.item_types) ? parsed.item_types : [],
    };
  } catch {
    return { itemTypes: [] };
  }
}

export default function Settings() {
  const [integrations, setIntegrations] = useState<Integration[]>([]);
  const [availableIntegrations, setAvailableIntegrations] = useState<Integration[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isDetecting, setIsDetecting] = useState(false);

  useEffect(() => {
    const load = async () => {
      try {
        const [allIntegrations, available] = await Promise.all([
          listIntegrations(),
          getAvailableIntegrations(),
        ]);

        if (allIntegrations.length === 0) {
          const detected = await detectIntegrations();
          setIntegrations(detected);
          setAvailableIntegrations(detected.filter((integration) => integration.is_available));
        } else {
          setIntegrations(allIntegrations);
          setAvailableIntegrations(available);
        }
      } catch (error) {
        toast.error("Failed to load settings", {
          description:
            error instanceof Error ? error.message : "Unknown error occurred",
        });
      } finally {
        setIsLoading(false);
      }
    };

    load();
  }, []);

  const handleDetectIntegrations = async () => {
    try {
      setIsDetecting(true);
      const detected = await detectIntegrations();
      setIntegrations(detected);
      const available = await getAvailableIntegrations();
      setAvailableIntegrations(available);
      toast.success("Integration scan complete");
    } catch (error) {
      toast.error("Failed to detect integrations", {
        description:
          error instanceof Error ? error.message : "Unknown error occurred",
      });
    } finally {
      setIsDetecting(false);
    }
  };

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="h-12 w-12 animate-spin rounded-full border-b-2 border-primary" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-semibold tracking-tight">Settings</h1>

      <Tabs defaultValue="integrations">
        <TabsList className="grid grid-cols-2 mb-6">
          <TabsTrigger value="integrations">Integrations</TabsTrigger>
          <TabsTrigger value="data">Data</TabsTrigger>
        </TabsList>

        <TabsContent value="integrations">
          <Card>
            <CardHeader>
              <CardTitle>Integration Defaults</CardTitle>
              <CardDescription>
                Detected tool availability on this device.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-end">
                <p className="mr-auto text-xs text-muted-foreground">
                  {`${availableIntegrations.length}/${integrations.length} available`}
                </p>
                <Button
                  variant="outline"
                  disabled={isDetecting}
                  onClick={handleDetectIntegrations}
                >
                  {isDetecting ? "Detecting..." : "Detect Again"}
                </Button>
              </div>

              <div className="rounded-md border">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b">
                      <th className="px-3 py-2 text-left font-medium">Tool</th>
                      <th className="px-3 py-2 text-left font-medium">Method</th>
                      <th className="px-3 py-2 text-left font-medium">Command</th>
                      <th className="px-3 py-2 text-left font-medium">Capabilities</th>
                      <th className="px-3 py-2 text-left font-medium">Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {integrations.length === 0 ? (
                      <tr>
                        <td
                          colSpan={5}
                          className="px-3 py-3 text-muted-foreground"
                        >
                          No integration records yet.
                        </td>
                      </tr>
                    ) : (
                      integrations.map((integration) => {
                        const capabilities = parseCapabilities(integration.metadata_json);
                        return (
                          <tr key={integration.id} className="border-b last:border-b-0">
                            <td className="px-3 py-2">{integration.display_name}</td>
                            <td className="px-3 py-2 text-muted-foreground">
                              {integration.detection_method ?? "n/a"}
                            </td>
                            <td className="px-3 py-2 text-muted-foreground">
                              {integration.launch_command ?? "builtin"}
                            </td>
                            <td className="px-3 py-2 text-xs text-muted-foreground">
                              {capabilities.itemTypes.length > 0
                                ? capabilities.itemTypes.join("/")
                                : "n/a"}
                            </td>
                            <td className="px-3 py-2">
                              <span
                                className={
                                  integration.is_available
                                    ? "text-emerald-600"
                                    : "text-muted-foreground"
                                }
                              >
                                {integration.is_available ? "Available" : "Unavailable"}
                              </span>
                            </td>
                          </tr>
                        );
                      })
                    )}
                  </tbody>
                </table>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="data">
          <Card>
            <CardHeader>
              <CardTitle>Data and Backups</CardTitle>
              <CardDescription>
                Local data and backup controls will be migrated to the new persistence model next.
              </CardDescription>
            </CardHeader>
            <CardContent className="text-sm text-muted-foreground">
              Current schema and command foundations are complete. Backup workflow migration is pending.
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
