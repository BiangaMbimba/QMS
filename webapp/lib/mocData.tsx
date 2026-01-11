

export interface HistoryItem {
    id: number,
    ticket_number: number,
    desk_name: String,
    created_at: String,
}

export interface Device {
    id: number;
    name: string;
    ipAddress?: string;
    status?: "connected" | "disconnected";
    token: string;
}

export interface Announcement {
    id: string;
    message: string;
    active: boolean;
}

export interface TicketCall {
  guichet: string;
  compteur: number;
}