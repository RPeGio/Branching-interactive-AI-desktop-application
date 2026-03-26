export interface BalanceMessage {
    available: boolean,
    balance: string | null,
    currency: string | null,
}

export interface HistoryItem {
    id: number;
    title: string;
    date: string;
    contexts: ContextItem[];
}

export interface ContextItem {
    role: string;
    content: string;
}

export interface ModelConfig {
    temperature: number,
    max_tokens: number,
    top_p: number,
    frequency_penalty: number,
}