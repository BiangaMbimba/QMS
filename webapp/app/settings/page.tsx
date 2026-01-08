"use client"

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { cn } from "@/lib/utils";
import { Copy, Delete, Key, Plus, Settings2, Wifi, WifiOff } from "lucide-react";
import { useState, useEffect } from "react";
import { toast } from "sonner";
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { execPath } from "process";
import { error } from "console";


export interface Device {
    id: number;
    name: string;
    ipAddress?: string;
    status?: "connected" | "disconnected";
    token: string;
}

export default function Settings() {

    const [devices, setDevices] = useState<Device[]>([]);
    const [newDeviceName, setNewDeviceName] = useState("");
    const [newDeviceIP, setNewDeviceIP] = useState("");
    const [soundEnabled, setSoundEnabled] = useState(true);

    const [ipAddr, setIpAddr] = useState<string>("");

    const registerDevice = async () => {

        if (newDeviceName.length > 3) {
            const register = await invoke("register_device", { name: newDeviceName });
            console.log(register);
            setNewDeviceName("");
        }
        else {
            alert("Device name must contain at least 3 characters");
        }
    }

    const handleCopy = (text: string | null) => {
        if (!text) return; 
        navigator.clipboard.writeText(text);
        toast.info("Copier", {description: "Text is succefully copied !"})
    };

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

        async function getAlldevices() {
            try {
                const get_devices = await invoke<Device[]>("get_all_devices");
                console.log(get_devices);
                setDevices(get_devices);
            } catch (e) {
                console.log(e);
            }

        }

        getIpAddr();
        getAlldevices();

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

                                        <div className="flex items-center gap-2">
                                            <code className="text-xs bg-muted px-2 py-1 rounded font-mono">
                                                {device.token.substring(0, 20)}...
                                            </code>
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                className="h-7 w-7"
                                                onClick={() => handleCopy(device.token)}
                                            >
                                                <Copy className="w-3 h-3" />
                                            </Button>
                                        </div>

                                    </TableCell>
                                    <TableCell className="text-right">
                                        <Button
                                            variant="outline"
                                            size="sm"
                                            className="gap-2"
                                        >
                                            <Delete className="w-3 h-3" />
                                            Delete
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
                        <Button type="submit" className="gap-2" onClick={registerDevice}>
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