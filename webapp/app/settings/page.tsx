"use client"

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { cn } from "@/lib/utils";
import { Copy, Key, Plus, Settings2, Wifi, WifiOff } from "lucide-react";
import { useState, useEffect } from "react";
import { toast } from "sonner";
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { execPath } from "process";
import { error } from "console";


export interface Device {
    id: string;
    name: string;
    ipAddress: string;
    status: "connected" | "disconnected";
    lastSeen: Date;
    authToken?: string;
}

export const devicesData: Device[] = [
    { id: "dev-001", name: "Counter Terminal 01", ipAddress: "192.168.1.101", status: "connected", lastSeen: new Date() },
    { id: "dev-002", name: "Counter Terminal 02", ipAddress: "192.168.1.102", status: "connected", lastSeen: new Date() },
    { id: "dev-003", name: "Ticket Printer Main", ipAddress: "192.168.1.150", status: "connected", lastSeen: new Date() },
    { id: "dev-004", name: "Display Screen Lobby", ipAddress: "192.168.1.200", status: "disconnected", lastSeen: new Date(Date.now() - 3600000) },
    { id: "dev-005", name: "Counter Terminal 03", ipAddress: "192.168.1.103", status: "connected", lastSeen: new Date() },
];

export function generateAuthToken(): string {
    const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let token = "";
    for (let i = 0; i < 16; i++) {
        token += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return token;
}


export default function Settings() {

    const [devices, setDevices] = useState<Device[]>(devicesData);
    const [newDeviceName, setNewDeviceName] = useState("");
    const [newDeviceIP, setNewDeviceIP] = useState("");
    const [soundEnabled, setSoundEnabled] = useState(true);

    const [ipAddr, setIpAddr] = useState<string>("");

    const handleGenerateTocken = async (deviceId: string) => {
        try {
            const token = await invoke<string>("generate_token", { device: deviceId });
            setDevices((prev) =>
                prev.map((d) => (d.id === deviceId ? { ...d, authToken: token } : d))
            );
            toast("Token Generated", {
                description: "New authentication token has been generated successfully.",
            });
        } catch (e) {
            console.error(e);
        }
    }

    useEffect(() => {
        async function getIpAddr() {
            try {
                const machineIp = await invoke<string>("get_machine_ip");
                console.log(machineIp);

                setIpAddr(machineIp);
            } catch (e) {
                console.log('Error while getting ip address ... ');

            }
        }

        getIpAddr();

    }, [])

    return (
        <main className="p-8 space-y-8 animate-fade-in w-full">
            <div>
                <h1 className="text-3xl font-bold text-foreground">Settings & Security</h1>
                <p className="text-muted-foreground mt-1">Manage devices, authentication, and system preferences</p>
            </div>

            <Card>
                <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                        <Key className="w-5 h-5 text-accent" />
                        Device Management
                    </CardTitle>
                    <CardDescription>
                        Manage connected devices and generate secure authentication tokens
                    </CardDescription>
                </CardHeader>

                <CardContent>
                    <Table>
                        <TableHeader>
                            <TableRow className="bg-muted/50">
                                <TableHead className="font-semibold">Device Name</TableHead>
                                <TableHead className="font-semibold">IP Address</TableHead>
                                <TableHead className="font-semibold">Status</TableHead>
                                <TableHead className="font-semibold">Auth Token</TableHead>
                                <TableHead className="font-semibold text-right">Actions</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {devices.map((device) => (
                                <TableRow key={device.id} className="hover:bg-muted/30 transition-colors">
                                    <TableCell className="font-medium">{device.name}</TableCell>
                                    <TableCell className="font-mono text-sm text-muted-foreground">
                                        {device.ipAddress}
                                    </TableCell>
                                    <TableCell>
                                        <Badge
                                            variant="outline"
                                            className={cn(
                                                "gap-1",
                                                device.status === "connected"
                                                    ? "border-success text-success"
                                                    : "border-muted-foreground text-muted-foreground"
                                            )}
                                        >
                                            {device.status === "connected" ? (
                                                <Wifi className="w-3 h-3" />
                                            ) : (
                                                <WifiOff className="w-3 h-3" />
                                            )}
                                            {device.status}
                                        </Badge>
                                    </TableCell>
                                    <TableCell>
                                        {device.authToken ? (
                                            <div className="flex items-center gap-2">
                                                <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                                                    {device.authToken.substring(0, 20)}...
                                                </code>
                                                <Button
                                                    variant="ghost"
                                                    size="icon"
                                                    className="h-7 w-7"

                                                >
                                                    <Copy className="w-3 h-3" />
                                                </Button>
                                            </div>
                                        ) : (
                                            <span className="text-muted-foreground text-sm">No token generated</span>
                                        )}
                                    </TableCell>
                                    <TableCell className="text-right">
                                        <Button
                                            variant="outline"
                                            size="sm"
                                            onClick={() => handleGenerateTocken(device.id)}
                                            className="gap-2"
                                        >
                                            <Key className="w-3 h-3" />
                                            Generate Token
                                        </Button>
                                    </TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                        <Plus className="w-5 h-5 text-accent" />
                        Add New Device
                    </CardTitle>
                    <CardDescription>
                        Register a new terminal or display device to the system
                    </CardDescription>
                </CardHeader>
                <CardContent>
                    <form className="flex gap-4 items-end">
                        <div className="flex-1 space-y-2">
                            <Label htmlFor="deviceName">Device Name</Label>
                            <Input
                                id="deviceName"
                                placeholder="e.g., Counter Terminal 04"
                                value={newDeviceName}
                                onChange={(e) => setNewDeviceName(e.target.value)}
                            />
                        </div>
                        <div className="flex-1 space-y-2">
                            <Label htmlFor="ipAddress">IP Address</Label>
                            <Input
                                id="ipAddress"
                                placeholder="e.g., 192.168.1.104"
                                value={newDeviceIP}
                                onChange={(e) => setNewDeviceIP(e.target.value)}
                            />
                        </div>
                        <Button type="submit" className="gap-2">
                            <Plus className="w-4 h-4" />
                            Add Device
                        </Button>
                    </form>
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                        <Settings2 className="w-5 h-5 text-accent" />
                        Global Settings
                    </CardTitle>
                    <CardDescription>
                        Configure system-wide preferences and organization details
                    </CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">
                    <div className="flex items-center justify-between">
                        <div className="space-y-0.5">
                            <Label htmlFor="sound" className="text-base">Enable Sound Notifications</Label>
                            <p className="text-sm text-muted-foreground">
                                Play audio alerts when a new ticket is called
                            </p>
                        </div>
                        <Switch
                            id="sound"
                            checked={soundEnabled}
                            onCheckedChange={setSoundEnabled}
                        />
                    </div>

                    {/* <div className="border-t border-border pt-6 space-y-2">
            <Label htmlFor="orgName" className="flex items-center gap-2">
              <Building2 className="w-4 h-4" />
              Organization Name
            </Label>
            <div className="flex gap-4">
              <Input
                id="orgName"
                value={orgName}
                onChange={(e) => setOrgName(e.target.value)}
                className="max-w-md"
              />
              <Button
                variant="outline"
                onClick={() =>
                  toast({
                    title: "Settings Saved",
                    description: "Organization name has been updated.",
                  })
                }
              >
                Save Changes
              </Button>
            </div>
          </div> */}
                </CardContent>
            </Card>
        </main>
    )

}