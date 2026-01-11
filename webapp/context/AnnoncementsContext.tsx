'use client';

import { Announcement } from '@/lib/mocData';
import { invoke } from '@tauri-apps/api/core';
import { createContext, Dispatch, SetStateAction, useCallback, useContext, useEffect, useState } from 'react';

interface AnnouncementsContextType {
    announcements: Announcement[];
    setAnnouncements: Dispatch<SetStateAction<Announcement[]>>;
    refreshAnnouncements: () => Promise<void>;
}

const AnnouncementsContext = createContext<AnnouncementsContextType | undefined>(undefined);

export const useAnnouncementContext = () => {
    const context = useContext(AnnouncementsContext);
    if (!context) {
        throw new Error("useAnnouncementContext must be used within an AnnouncementsProvider");
    }
    return context;
};

export const AnnouncementsProvider = ({ children }: { children: React.ReactNode }) => {
    const [announcements, setAnnouncements] = useState<Announcement[]>([]);

    const refreshAnnouncements = useCallback(async () => {
        try {
            const data = await invoke<Announcement[]>("get_annonces");
            setAnnouncements(data);
        } catch (error) {
            console.error("Failed to fetch announcements:", error);
        }
    }, []);

    useEffect(() => {
        refreshAnnouncements();
    }, [refreshAnnouncements]);

    return (
        // 4. Expose the refresh function to the children
        <AnnouncementsContext.Provider value={{ announcements, setAnnouncements, refreshAnnouncements }}>
            {children}
        </AnnouncementsContext.Provider>
    );
}