'use client';

import { createContext, useContext, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

interface TauriData {
  guichet: string;
  compteur: number;
}

const defaultState: TauriData = {
  guichet: "",
  compteur: 0
};

const TauriContext = createContext<TauriData>(defaultState);

export const useTauriEvents = () => useContext(TauriContext);

export function TauriProvider({ children }: { children: React.ReactNode }) {
  const [data, setData] = useState<TauriData>(defaultState);

//   const annonce = (text: string) => {
//     window.speechSynthesis.cancel();

//     const utterance = new SpeechSynthesisUtterance(text);

//     utterance.lang = 'fr-FR'; // Set language to French
//     utterance.rate = 0.9;      // Speed (0.1 to 10)
//     utterance.pitch = 1;       // Pitch (0 to 2)

//     window.speechSynthesis.speak(utterance);
//   }

  useEffect(() => {

    let unlisten: (() => void) | undefined;

    const setLastState = async () => {
        const get_state = await invoke<TauriData>("get_counter_state");
        setData(get_state);

        console.log("Last state: ", get_state);
    }

    const setupListener = async () => {
      unlisten = await listen<TauriData>('nouveau-message', (event) => {
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