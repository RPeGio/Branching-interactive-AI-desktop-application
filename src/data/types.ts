export interface BalanceMessage {
    available: boolean,
    balance: string | null,
    currency: string | null,
}

export interface HistoryItem {
    id: number,
    title: string,
    date: string,
    config: ModelParamsForServer,
    contexts: ContextItem[],
}

export interface ContextItem {
    role: string,
    content: string,
}

export interface GlobalUserConfig {
    globalSystemPrompt: string,
}

export interface ConfigItem {
    systemPrompt: string,
    temperature: number,
    maxTokens: number,
    topP: number,
    frequencyPenalty: number,
}

export interface ModelParamsForServer {
    temperature: number,
    max_tokens: number,
    top_p: number,
    frequency_penalty: number,
}