'use client';

import { Maximize2, Minimize2 } from 'lucide-react';
import { useState, useEffect, useRef } from 'react';
import { invoke } from "@tauri-apps/api/core"
import { listen } from '@tauri-apps/api/event';

export interface TicketCall {
    ticketNumber: string;
    counter: string;
    timestamp: Date;
}

// History of last calls
export const callHistory: TicketCall[] = [
    { ticketNumber: "041", counter: "COUNTER 03", timestamp: new Date() },
    { ticketNumber: "040", counter: "COUNTER 01", timestamp: new Date() },
    { ticketNumber: "015", counter: "COUNTER 07", timestamp: new Date() },
    { ticketNumber: "039", counter: "COUNTER 02", timestamp: new Date() },
    { ticketNumber: "008", counter: "COUNTER 04", timestamp: new Date() },
    { ticketNumber: "038", counter: "COUNTER 06", timestamp: new Date() },
];


export default function Display() {

    const [isFullscreen, setIsFullscreen] = useState(false);
    const mainRef = useRef(null);

    const [logs, setLogs] = useState<string[]>([]);

    const toggleFullScreen = async () => {
        try {
            if (!document.fullscreenElement) {
                if (mainRef.current) {
                    await mainRef.current.requestFullscreen();
                    setIsFullscreen(true);
                }
            } else {
                if (document.exitFullscreen) {
                    await document.exitFullscreen();
                    setIsFullscreen(false);
                }
            }
        } catch (err) {
            console.error("Error attempting to enable fullscreen:", err);
        }
    };

    useEffect(() => {
        const unlistenPromise = listen<string>('nouveau-message', (event) => {
            console.log(event.payload);
            // setLogs((prev) => [...prev, `Reçu : ${event.payload}`]);
        });

        return () => {
            unlistenPromise.then((f) => f());
        };
    }, []);

    return (
        <>
            <main className="flex-1 flex flex-col relative" ref={mainRef}>
                <div className="flex-1 flex bg-sidebar-foreground">
                    <div className="flex-1 flex flex-col items-center justify-center p-8 relative overflow-hidden">
                        <div className="relative z-10 display-glow rounded-2xl border-2 border-primary py-10 px-20 mb-8 flex flex-col gap-8 min-w-[350px]">
                            <p className="ticket-number text-[10rem] font-bold text-display-number leading-none pulse-call text-center">
                                120
                            </p>
                        </div>
                        <div className="relative z-10 flex flex-col items-center">
                            <span className="text-xl text-foreground/60 uppercase tracking-wider mb-2">
                                Please proceed to
                            </span>
                            <span className="text-5xl font-bold text-foreground tracking-wide">
                                COUNTER 01
                            </span>
                        </div>
                    </div>
                    <div className="w-[25%] bg-primary/90 border-l border-display-accent/20 p-6 flex flex-col">
                        <div className="mb-6">
                            <h2 className="text-xl font-semibold text-display-foreground tracking-wide uppercase">
                                Recently Called
                            </h2>
                            <div className="h-1 w-16 bg-display-accent mt-2 rounded-full" />
                        </div>
                        <div className="flex-1 space-y-3">
                            {callHistory.map((call, index) => (
                                <div
                                    key={index}
                                    className="flex items-center justify-between bg-display/50 border border-display-accent/10 rounded-lg p-4 animate-fade-in"
                                    style={{ animationDelay: `${index * 100}ms` }}
                                >
                                    <span className="ticket-number text-2xl font-bold text-display-accent">
                                        {call.ticketNumber}
                                    </span>
                                    <span className="text-lg font-medium text-display-foreground/80">
                                        {call.counter}
                                    </span>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
                <div className="h-14 bg-primary w-full order-t border-display-accent/20 flex items-center overflow-hidden">
                    <div className="flex items-center px-4 bg-display-accent text-display-foreground font-semibold h-full">
                        <span className="uppercase tracking-wider text-sm">Announcements</span>
                    </div>
                    <div className="flex-1 overflow-hidden relative">
                        <div className="ticker-scroll whitespace-nowrap text-display-foreground/90 text-lg">
                            <span className="mx-20">L'app est creee pour la gestion de l'attendes de client</span>
                            <span className="mx-16">Remile Bianga</span>
                            <span className="mx-16">Remile Bianga</span>
                            <span className="mx-16">Remile Bianga</span>
                        </div>
                    </div>
                </div>
                <div className="absolute top-4 left-8 p-4 bg-primary/70 rounded-lg text-sidebar-foreground flex gap-2 hover:cursor-pointer hover:bg-primary transition-all duration-200" onClick={toggleFullScreen}>
                    {
                        !isFullscreen
                            ? <>
                                <Maximize2 />
                                <p>Full screen</p>
                            </>
                            : <Minimize2 />
                    }
                </div>
            </main>
        </>
    );
}
