"use client"

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { cn } from "@/lib/utils";
import { Check, Copy, Delete, Key, Megaphone, Palette, Pencil, Plus, Settings2, Trash2, Wifi, WifiOff, X } from "lucide-react";
import { useState, useEffect } from "react";
import { toast } from "sonner";
import { invoke } from '@tauri-apps/api/core';
import { Textarea } from "@/components/ui/textarea";
import { Announcement, Device } from "@/lib/mocData";
import { useAnnouncementContext } from "@/context/AnnoncementsContext";

export default function Settings() {

    const [devices, setDevices] = useState<Device[]>([]);
    const [newDeviceName, setNewDeviceName] = useState("");
    const [newDeviceIP, setNewDeviceIP] = useState("");
    const [soundEnabled, setSoundEnabled] = useState(true);

    const [ipAddr, setIpAddr] = useState<string>("");

    const { announcements, setAnnouncements, refreshAnnouncements } = useAnnouncementContext();
    const [newAnnouncement, setNewAnnouncement] = useState("");
    const [editingId, setEditingId] = useState<string | null>(null);
    const [editingText, setEditingText] = useState("");

    async function getAlldevices() {
        try {
            const get_devices = await invoke<Device[]>("get_all_devices");
            console.log(get_devices);
            setDevices(get_devices);
        } catch (e) {
            console.log(e);
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
                                                <><WifiOff className="w-3 h-3" /> disconnected</>
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
                                                onClick={() => {
                                                    if (!device.token) return;
                                                    navigator.clipboard.writeText(device.token);
                                                    toast.info("Copier", { description: "Text is succefully copied !" })
                                                }}
                                            >
                                                <Copy className="w-3 h-3" />
                                            </Button>
                                        </div>

                                    </TableCell>
                                    <TableCell className="text-right">
                                        <Button
                                            variant="outline"
                                            size="sm"
                                            className="gap-2 text-destructive hover:text-destructive"
                                            onClick={async () => {
                                                try {
                                                    await invoke("delete_device", { id: device.id })
                                                    getAlldevices();
                                                    toast.success("Device deleted", {
                                                        description: "Device has been deleted."
                                                    })
                                                } catch (e) {
                                                    console.error(e);
                                                    toast.error("Fail to delete", {
                                                        description: "Decive has not been deleted ..."
                                                    })
                                                }

                                            }}
                                        >
                                            <Trash2 className="w-3 h-3" />
                                        </Button>
                                    </TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                    {devices.length === 0 && (
                        <p className="text-center text-muted-foreground py-4">
                            No device yet. Add one below.
                        </p>
                    )}
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
                        <Button type="submit" className="gap-2" onClick={async (e) => {
                            e.preventDefault();
                            if (newDeviceName.trim().length > 3) {
                                try {
                                    await invoke("register_device", { name: newDeviceName.trim() });
                                    getAlldevices();
                                    toast.success("Device added", {
                                        description: "Device has been added."
                                    })
                                    setNewDeviceName("");
                                } catch (e) {
                                    console.error(e);
                                    toast.error("Fail to add", {
                                        description: "Fail to add a device ..."
                                    })

                                }
                            }
                            else {
                                toast.info("Fail to add", {
                                    description: "Device name must be more than 3 characters ..."
                                })
                            }
                        }}>
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
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle className="flex items-center gap-2">
                        <Megaphone className="w-5 h-5 text-accent" />
                        Announcements
                    </CardTitle>
                    <CardDescription>
                        Manage ticker announcements displayed on the public screen
                    </CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                    {/* Add New Announcement */}
                    <form
                        onSubmit={async (e) => {
                            e.preventDefault();
                            if (!newAnnouncement.trim()) {
                                toast.error("Error", {
                                    description: "Please enter an announcement.",
                                });
                                return;
                            }
                            const newItem: Announcement = {
                                id: `ann-${Date.now()}`,
                                message: newAnnouncement.trim(),
                                active: true,
                            };
                            setAnnouncements((prev) => [...prev, newItem]);
                            try {
                                await invoke("add_annonce", { message: newItem.message });
                                refreshAnnouncements();
                                setNewAnnouncement("");
                                toast.success("Announcement Added", {
                                    description: "New announcement has been added to the ticker.",
                                });

                            } catch (e) {
                                console.error(e);
                                toast.error("Failed to add", {
                                    description: "New announcement has not been added ...",
                                });
                            }
                        }}
                        className="flex gap-4 items-end"
                    >
                        <div className="flex-1 space-y-2">
                            <Label htmlFor="newAnnouncement">New Announcement</Label>
                            <Textarea
                                id="newAnnouncement"
                                placeholder="Enter announcement text..."
                                value={newAnnouncement}
                                onChange={(e) => setNewAnnouncement(e.target.value)}
                                className="min-h-[60px]"
                            />
                        </div>
                        <Button type="submit" className="gap-2">
                            <Plus className="w-4 h-4" />
                            Add
                        </Button>
                    </form>

                    {/* Announcements List */}
                    <div className="border-t border-border pt-4 space-y-2">
                        {announcements.map((announcement) => (
                            <div
                                key={announcement.id}
                                className={cn(
                                    "flex items-center gap-3 p-3 rounded-lg border transition-colors",
                                    announcement.active ? "bg-muted/30 border-border" : "bg-muted/10 border-muted"
                                )}
                            >
                                <Switch
                                    checked={announcement.active}
                                    onCheckedChange={async (checked) => {
                                        setAnnouncements((prev) =>
                                            prev.map((a) =>
                                                a.id === announcement.id ? { ...a, active: checked } : a
                                            )
                                        );

                                        try {
                                            await invoke("set_annonce_active", { id: announcement.id, isActive: checked });
                                            refreshAnnouncements();
                                        } catch (e) {
                                            console.error(e);
                                        }
                                    }}
                                />
                                {editingId === announcement.id ? (
                                    <div className="flex-1 flex gap-2">
                                        <Textarea
                                            value={editingText}
                                            onChange={(e) => setEditingText(e.target.value)}
                                            className="min-h-[40px] flex-1"
                                            autoFocus
                                        />
                                        <Button
                                            size="icon"
                                            variant="ghost"
                                            className="h-8 w-8 text-success hover:text-success"
                                            onClick={async () => {
                                                setAnnouncements((prev) =>
                                                    prev.map((a) =>
                                                        a.id === announcement.id ? { ...a, message: editingText } : a
                                                    )
                                                );

                                                try {
                                                    await invoke("update_annonce_message", { id: announcement.id, message: editingText });
                                                    refreshAnnouncements();
                                                    setEditingId(null);
                                                    toast.success("Announcement Updated", {
                                                        description: "Changes have been saved.",
                                                    });
                                                } catch (e) {
                                                    console.error(e);
                                                    toast.error("Failed to update", {
                                                        description: "Changes have not been saved ...",
                                                    });
                                                }
                                            }}
                                        >
                                            <Check className="w-4 h-4" />
                                        </Button>
                                        <Button
                                            size="icon"
                                            variant="ghost"
                                            className="h-8 w-8"
                                            onClick={() => setEditingId(null)}
                                        >
                                            <X className="w-4 h-4" />
                                        </Button>
                                    </div>
                                ) : (
                                    <>
                                        <p className={cn(
                                            "flex-1 text-sm",
                                            !announcement.active && "text-muted-foreground"
                                        )}>
                                            {announcement.message}
                                        </p>
                                        <Button
                                            size="icon"
                                            variant="ghost"
                                            className="h-8 w-8"
                                            onClick={() => {
                                                setEditingId(announcement.id);
                                                setEditingText(announcement.message);
                                            }}
                                        >
                                            <Pencil className="w-4 h-4" />
                                        </Button>
                                        <Button
                                            size="icon"
                                            variant="ghost"
                                            className="h-8 w-8 text-destructive hover:text-destructive"
                                            onClick={async () => {
                                                setAnnouncements((prev) =>
                                                    prev.filter((a) => a.id !== announcement.id)
                                                );

                                                try {
                                                    await invoke("delete_annonce", { id: announcement.id });
                                                    refreshAnnouncements();
                                                    toast.success("Announcement Deleted", {
                                                        description: "The announcement has been removed.",
                                                    });
                                                } catch (e) {
                                                    console.error(e);
                                                    toast.error("Failed to delete", {
                                                        description: "The announcement has not been removed ...",
                                                    });
                                                }
                                            }}
                                        >
                                            <Trash2 className="w-4 h-4" />
                                        </Button>
                                    </>
                                )}
                            </div>
                        ))}
                        {announcements.length === 0 && (
                            <p className="text-center text-muted-foreground py-4">
                                No announcements yet. Add one above.
                            </p>
                        )}
                    </div>
                </CardContent>
            </Card>
        </main>
    )

}