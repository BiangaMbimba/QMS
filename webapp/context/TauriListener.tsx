'use client';

import { createContext, useContext, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { TicketCall } from '@/lib/mocData';

const defaultState: TicketCall = {
  guichet: "",
  compteur: 0
};

const TauriContext = createContext<TicketCall>(defaultState);

export const useTauriEvents = () => useContext(TauriContext);

export function TauriProvider({ children }: { children: React.ReactNode }) {
  const [data, setData] = useState<TicketCall>(defaultState);

  useEffect(() => {

    let unlisten: (() => void) | undefined;

    const setLastState = async () => {
        const get_state = await invoke<TicketCall>("get_counter_state");
        setData(get_state);

        console.log("Last state: ", get_state);
    }

    const setupListener = async () => {
      unlisten = await listen<TicketCall>('nouveau-message', (event) => {
        console.log('Global Event:', event.payload);
        setData(event.payload);
        // const message = `Client numero ${event.payload.compteur}, au guichet ${event.payload.guichet}`;
      });
    };

    setLastState();
    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  return (
    <TauriContext.Provider value={data}>
      {children}
    </TauriContext.Provider>
  );
}