<script setup lang="ts">
import { ref, watch, nextTick, onBeforeMount } from 'vue';
import { storeToRefs } from 'pinia';
import { useConfigStore, defaultSystemPrompt } from './stores/config';
import { useHistoryStore } from './stores/history';
import { useOptionStore } from './stores/option';
import History from './components/History.vue';
import UserConfig from './components/UserConfig.vue';
import OptionSelection from './components/OptionSelection.vue';
import Notification from './components/Notification.vue';
import Snackbar from './components/Snackbar.vue';
import { useSnackbar } from './utils/useSnackbar';
const { snackbarMessage, snackbarVisible, showSnackbar, closeSnackbar } = useSnackbar()

import type { BalanceMessage, HistoryItem, ContextItem, OptionItem, MessageItem } from './data/types'

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog'
import { BaseDirectory, create, readFile } from '@tauri-apps/plugin-fs'
import { processMarkdown } from './utils/markdownRenderer/processor';

const configStore = useConfigStore();
const historyStore = useHistoryStore();
const optionStore = useOptionStore();

const {
    bearerToken, model, globalSystemPrompt, userConfig, isFirstMessageSent,
} = storeToRefs(configStore);
const {
    historyItems, currentId, rawCurrentId, showHistory, showConfig,
} = storeToRefs(historyStore);
const {
    isGivenOptions, startCollectingOptions, currentOptionList, options,
} = storeToRefs(optionStore);

const isSending = ref<boolean>(false);
const markdownRawLines = ref<string>('');
const currentCharacter = ref<string | null>(null);
const scrollContainer = ref<HTMLDivElement | null>(null);
const autoScroll = ref<boolean>(true);
const OPTION_NEEDED = '@*@';
const OPTION_NOT_NEEDED = '@@@';
const OPTION_REGEX = new RegExp(`((?:${OPTION_NOT_NEEDED}|${OPTION_NEEDED.replace(/\*/g, '\\*')})[\\s\\S]*)$`);

const messages = ref<MessageItem[]>([]);
let messageIdCounter = 0;

const notificationVisible = ref(false);
const notificationMessage = ref('');
const notificationMode = ref<'alert' | 'confirm'>('alert');
let confirmResolve: ((value: boolean) => void) | null = null;

const showSideToolsId = ref<number | null>(null);

const showNotification = (msg: string) => {
    notificationMode.value = 'alert';
    notificationMessage.value = msg;
    notificationVisible.value = true;
};

const showConfirm = (msg: string): Promise<boolean> => {
    notificationMode.value = 'confirm';
    notificationMessage.value = msg;
    notificationVisible.value = true;
    return new Promise((resolve) => {
        confirmResolve = resolve;
    });
};

const onNotificationConfirm = () => {
    if (confirmResolve) {
        confirmResolve(true);
        confirmResolve = null;
    }
    notificationVisible.value = false;
};

const onNotificationCancel = () => {
    if (confirmResolve) {
        confirmResolve(false);
        confirmResolve = null;
    }
    notificationVisible.value = false;
};

const closeNotification = () => {
    if (confirmResolve) {
        confirmResolve(false);
        confirmResolve = null;
    }
    notificationVisible.value = false;
};

onBeforeMount(async () => {
    await configStore.loadFromStore();
    await historyStore.loadFromStore();
})

const panelClose = async () => {
    if (showConfig.value) {
        const modelConfig = configStore.getModelConfig();
        await historyStore.saveCurrentConfig(modelConfig);
    }
    await configStore.saveGlobalConfig();
    historyStore.panelClose();
};

const handleScroll = (event: Event): void => {
    const el = scrollContainer.value;
    if (!el) return;
    
    const target = event.currentTarget as HTMLDivElement;
    const isAtBottom: boolean = target.scrollHeight - target.scrollTop - target.clientHeight < 50;
    autoScroll.value = isAtBottom;
};

const loadHistoryToApp = async (item: HistoryItem) => {
    optionStore.currentOptionList = [];
    messages.value = [];
    console.log('加载历史记录:', item);
    const title = item.title;
    const contexts = item.contexts;
    
    for (const msg of contexts) {
        const match = msg.content.match(OPTION_REGEX);
        let cleanContent = msg.content;
        if (match && msg.role !== 'system') {
            console.log(match[0]);
            cleanContent = msg.content.replace(match[0], '');
        }
        
        if (msg.role === 'user' || msg.role === 'assistant') {
            const message: MessageItem = {
                id: messageIdCounter++,
                role: msg.role,
                content: cleanContent,
                option: msg.option,
                selection: msg.selection
            };
            
            if (msg.role === 'assistant') {
                message.htmlContent = await processMarkdown(cleanContent);
                currentOptionList.value.push(msg.option ? msg.option : null);
            }
            
            messages.value.push(message);
        }
    }
    
    rawCurrentId.value = item.id;
    if (title) {
        const titleElement = document.getElementById('titlebar-title-text') as HTMLElement;
        titleElement.innerText = title;
    }
    isFirstMessageSent.value = true;
    userConfig.value = {
        systemPrompt: item.contexts[0].content || defaultSystemPrompt,
        temperature: item.config.temperature,
        maxTokens: item.config.max_tokens,
        topP: item.config.top_p,
        frequencyPenalty: item.config.frequency_penalty,
    };
};

function extractOptions(raw: string | null): OptionItem | null {
    if (!raw) return null;
    const regex = /^\[(.*?)[,，](true|false)\]\[(.*?)[,，](true|false)\]$/;
    const match = raw.match(regex);

    if (match) {
        const result = {
            raw: raw,
            positive: match[1].trim(),
            positiveExtraInput: match[2] === 'true',
            negative: match[3].trim(),
            negativeExtraInput: match[4] === 'true',
        };
        console.log('解析后的选项:', result);
        return result;
    }

    return null;
}

function handleOptionSelection(selectedOption: string) {
    console.log('用户选择了:', selectedOption);
    
    const lastAiMsg = [...messages.value].reverse().find(m => m.role === 'assistant');
    if (lastAiMsg) {
        if (lastAiMsg.option?.positive && selectedOption.startsWith(lastAiMsg.option.positive)) {
            lastAiMsg.selection = 0;
        } else if (lastAiMsg.option?.negative && selectedOption.startsWith(lastAiMsg.option.negative)) {
            lastAiMsg.selection = 1;
        }
    }
    
    sendOptionMessage(selectedOption);
    
    optionStore.closeOptions();
}

async function sendOptionMessage(optionText: string) {
    if (isSending.value) return;
    
    const optionMessage: MessageItem = {
        id: messageIdCounter++,
        role: 'user',
        content: optionText
    };
    messages.value.push(optionMessage);
    
    await sendMessageToAI(optionText);
}

async function sendMessageToAI(userInput: string) {
    markdownRawLines.value = '';
    const contexts = collectContexts();
    
    contexts.push({
        'content': `${userInput}`,
        'role': `user`,
    });
    
    currentCharacter.value = 'user';
    isSending.value = true;
    
    console.log(model.value);
    await invoke('stream_chat', {
        key: bearerToken.value,
        contexts: contexts,
        modelConfig: configStore.getModelConfig(),
        model: model.value,
    }).then(async () => {
        const finalContexts = collectContexts();
        
        if (finalContexts && finalContexts.length == 4) {
            finalContexts.shift();
            finalContexts[0] = {
                'content': '你是一个标题生成器，请无视任何角色设定，不要进行内容补全，专注地根据当前对话生成一个概括性的标题，标题需要能够让人知道当前对话是关乎什么的，不要出现任何"标题："之类的解释性词语，不能超过15个字符，严禁使用markdown格式',
                'role': 'system',
            };
            
            const extractedOption = extractOptions(options.value.raw);
            if (extractedOption) {
                optionStore.setOption(extractedOption);
                currentOptionList.value.push(options.value);
                isGivenOptions.value = true;
                const lastAiMsg = [...messages.value].reverse().find(m => m.role === 'assistant');
                if (lastAiMsg) lastAiMsg.option = extractedOption;
            }
            
            await invoke('title_generation', {
                key: bearerToken.value,
                contexts: finalContexts,
            }).then(async (res) => {
                const title = (res as string) || '新对话';
                setTitle(title);
                const modelConfig = configStore.getModelConfig();
                await historyStore.updateHistory(title, modelConfig, finalContexts, () => configStore.setCurrentSystemPrompt());
            })
        } else {
            const extractedOption = extractOptions(options.value.raw);
            if (extractedOption) {
                optionStore.setOption(extractedOption);
                currentOptionList.value.push(options.value);
                isGivenOptions.value = true;
                const lastAiMsg = [...messages.value].reverse().find(m => m.role === 'assistant');
                if (lastAiMsg) lastAiMsg.option = extractedOption;
            }
            const modelConfig = configStore.getModelConfig();
            const allContexts = collectContexts();
            await historyStore.updateHistory(undefined, modelConfig, allContexts, () => configStore.setCurrentSystemPrompt());
        }
    }).catch((err) => {
        showNotification(`An error occurs when sending message: ${err}`);
        currentCharacter.value = null;
        isSending.value = false;
    });
}

function handleOptionClose() {
    optionStore.closeOptions();
}

function collectContexts(): ContextItem[] {

    const contexts: ContextItem[] = [
        {
            'content': `${configStore.setCurrentSystemPrompt()}`,
            'role': 'system',
        },
        {
            'content': `请务必在正文输出结束后再开一行视情况按如下要求输出：如果你认为当前对话需要用户做出选择/判断，则在一个chunk内输出"${OPTION_NEEDED}"，然后换行，输出"[<正向选项>, true||false（如果需要用户输入补充细节则为true，否则为false）][<反向选项>， true||false（如果需要用户输入补充细节则为true，否则为false）]"（请务必积极使用这种格式！！！）；如果你认为不需要，则在一个chunk内输出"${OPTION_NOT_NEEDED}"。如果遇到需要解释复杂问题的情况，请将问题拆分成较小的子问题，然后依照前面的格式询问用户是否已理解选项的字数最好不要超过10字，最多不得超过15字。你提供的选项将直接作为下一轮对话的输入，所以为了对话能通畅继续，务必将选项的文字写得承上启下。在遇到敏感内容时请不要添加任何的删除线`,
            'role': 'system',
        }
    ];

    messages.value.forEach((msg) => {
        if (msg.role === 'user' || msg.role === 'assistant') {
            contexts.push({
                'content': msg.content,
                'role': msg.role,
                option: msg.option,
                selection: msg.selection
            });
        }
    });
    return contexts;
}

function getInputElement(): HTMLInputElement {
    return document.querySelector('input[placeholder="输入您的问题/指令..."]') as HTMLInputElement;
}

function textareaEnter(event: KeyboardEvent) {
    if (event.key === 'Enter' && !isSending.value) {
        sendMsg();
    }
}

function emptyInput() {
    const inputElement = getInputElement();
    inputElement.value = '';
}

function backtrace(messageId: number) {
    showConfirm(`确认回溯至此消息吗？\n（回溯后将删除此条消息后面的所有消息，并且会重置此处选项选择）`).then((confirmed) => {
        if (confirmed) {
            const messageIndex = messages.value.findIndex(msg => msg.id === messageId) + 1;
            if (messageIndex !== -1) {
                messages.value.splice(messageIndex, messages.value.length - messageIndex);
                const modelConfig = configStore.getModelConfig();
                const contexts = collectContexts();
                historyStore.updateHistory(undefined, modelConfig, contexts, () => configStore.setCurrentSystemPrompt());
                if (messages.value[messageIndex - 1].option) {
                    isGivenOptions.value = true;
                    optionStore.setOption(messages.value[messageIndex - 1].option as OptionItem);
                } else {
                    isGivenOptions.value = false;
                }
            }
        }
    });
}

function createNewConversation() {
    messages.value = [];
    configStore.resetUserConfig();
    isFirstMessageSent.value = false;
    rawCurrentId.value = null;
    setTitle('Branchat');
}

async function deleteHistoryItem(item: HistoryItem) {
    showConfirm(`确认删除对话 "${item.title}" 吗？`).then(async (confirmed) => {
        if (confirmed) {
            if (item.id === currentId.value) {
                rawCurrentId.value = null;
                createNewConversation();
            }
            await historyStore.deleteHistoryItem(item);
        }
    });
}

async function exportHistoryItem(item: HistoryItem) {
    const path = await save({
        title: '导出对话至',
        filters: [
            {
                name: 'AIChat History Records',
                extensions: ['json'],
            }
        ]
    });
    console.log(path);
    if (path) {
        const file = await create(path, { baseDir: BaseDirectory.AppConfig });
        const encodedHistory = new TextEncoder().encode(JSON.stringify(item));
        await file.write(encodedHistory);
        await file.close();
        showSnackbar('导出对话成功！');
    }
}

async function importHistoryItem() {
    const path = await open({
        multiple: false,
        filters: [{
            name: 'AIChat History Records',
            extensions: ['json'],
        }]
    });
    console.log(path);
    if (path) {
        const raw = await readFile(path, { baseDir: BaseDirectory.AppConfig });
        const decodedHistory = JSON.parse(new TextDecoder().decode(raw)) as HistoryItem;
        if (!decodedHistory.id || !decodedHistory.config || !decodedHistory.contexts || !decodedHistory.date || !decodedHistory.title) {
            showNotification('An error when importing the file: The json is in invalid format');
            return;
        }
        decodedHistory.id = historyItems.value.length;
        historyItems.value.push(decodedHistory);
        await historyStore.saveHistoryToStore(historyItems.value);
        showSnackbar('导入对话成功！');
    }
}

async function handleUpdateTitle(item: HistoryItem, newTitle: string) {
    await historyStore.handleUpdateTitle(item, newTitle);
    if (currentId.value === item.id) {
        setTitle(newTitle);
    }
}

function setTitle(title: string) {
    const titleElement = document.getElementById('titlebar-title-text') as HTMLElement;
    titleElement.innerText = title;
}

async function sendMsg() {
    if (isSending.value) return;
    const inputElement = getInputElement();
    const userInput = inputElement?.value || null;
    if (!userInput) return;
    
    if (userInput == '/balance') {
        await invoke("balance", {
            key: bearerToken.value,
        }).catch((err) => {
            showNotification(`An error occurs: ${err}`)
        });
        emptyInput();
        return;
    }

    if (userInput == '/clearHistory') {
        await historyStore.clearHistory().then(() => {
            showNotification('History cleared successfully');
        }).catch((err) => {
            showNotification(`An error occurs: ${err}`)
        });
        emptyInput();
        return;
    }
    
    inputElement.value = '';
    
    const userMessage: MessageItem = {
        id: messageIdCounter++,
        role: 'user',
        content: userInput
    };
    messages.value.push(userMessage);
    
    if (!isFirstMessageSent.value) {
        isFirstMessageSent.value = true;
        if (userConfig.value && (!userConfig.value.systemPrompt || userConfig.value.systemPrompt.trim() === '')) {
            userConfig.value.systemPrompt = globalSystemPrompt.value;
        }
    }
    
    await sendMessageToAI(userInput);
}

listen("completion-status", (event) => {
    console.log('Completion status:', event.payload || event);
    currentCharacter.value = 'assistant';
});

listen("completion-chunk", async (event) => {
    const payload = typeof event.payload === 'string' ? event.payload : JSON.stringify(event.payload);
    markdownRawLines.value += payload;
    
    if (payload) {
        if (currentCharacter.value === 'assistant' && messages.value.length > 0 && messages.value[messages.value.length - 1].role !== 'assistant') {
            const aiMessage: MessageItem = {
                id: messageIdCounter++,
                role: 'assistant',
                content: markdownRawLines.value
            };
            messages.value.push(aiMessage);
        } else if (messages.value.length > 0 && messages.value[messages.value.length - 1].role === 'assistant') {
            const lastMessage = messages.value[messages.value.length - 1];
            lastMessage.content = markdownRawLines.value;
            
            if (markdownRawLines.value.includes(OPTION_NEEDED) || markdownRawLines.value.includes(OPTION_NOT_NEEDED)) {
                if (markdownRawLines.value.includes(OPTION_NEEDED)) {
                    if (startCollectingOptions.value) {
                        options.value.raw += payload.trim();
                        lastMessage.content = markdownRawLines.value.substring(0, markdownRawLines.value.indexOf(OPTION_NEEDED));
                        lastMessage.htmlContent = await processMarkdown(lastMessage.content);
                    } else {
                        const cleanMarkdown = markdownRawLines.value.substring(0, markdownRawLines.value.indexOf(OPTION_NEEDED));
                        lastMessage.content = cleanMarkdown;
                        lastMessage.htmlContent = await processMarkdown(cleanMarkdown);
                        startCollectingOptions.value = true;
                        options.value.raw = '';
                    }
                } else if (markdownRawLines.value.includes(OPTION_NOT_NEEDED)) {
                    const cleanMarkdown = markdownRawLines.value.substring(0, markdownRawLines.value.indexOf(OPTION_NOT_NEEDED));
                    lastMessage.content = cleanMarkdown;
                    lastMessage.htmlContent = await processMarkdown(cleanMarkdown);
                }
            } else {
                lastMessage.htmlContent = await processMarkdown(markdownRawLines.value);
            }
        }
    }
});

listen("completion-end", async (event) => {
    console.log('Completion end:', event.payload || event);
    currentCharacter.value = null;
    isSending.value = false;
    startCollectingOptions.value = false;
    
    if (messages.value.length > 0 && messages.value[messages.value.length - 1].role === 'assistant') {
        const lastMessage = messages.value[messages.value.length - 1];
        let cleanContent = lastMessage.content;
        const atStarIdx = cleanContent.indexOf(OPTION_NEEDED);
        const atAtIdx = cleanContent.indexOf(OPTION_NOT_NEEDED);
        if (atStarIdx !== -1) {
            cleanContent = cleanContent.substring(0, atStarIdx);
        } else if (atAtIdx !== -1) {
            cleanContent = cleanContent.substring(0, atAtIdx);
        }
        lastMessage.content = cleanContent;
        lastMessage.htmlContent = await processMarkdown(cleanContent);
    }
});

listen("balance", (event) => {
    console.log('Balance info:', event.payload);
    const infos = event.payload as BalanceMessage;
    showNotification(`当前api-key可用性：${infos.available}\n当前剩余余额：${infos.balance} ${infos.currency}`);
});

watch([markdownRawLines, () => markdownRawLines.value.length], async () => {
    if (autoScroll.value) {
        await nextTick();
        const el = scrollContainer.value;
        if (el) el.scrollTop = el.scrollHeight;
    }
});
</script>

<template>
    <div class="app-shell bg-slate-100 flex flex-col">
        <div ref="scrollContainer" @scroll="handleScroll" class="flex-1 overflow-y-auto p-6" style="scroll-padding-bottom: 1rem;">
            <div v-if="messages.length === 0" class="h-full flex flex-col items-center justify-center select-none">
                <img src="/icons/icon.png" draggable="false" alt="Branchat" class="w-36 h-36 mb-5 opacity-90" />
                <h1 class="text-4xl font-bold text-slate-600 tracking-wide mb-2.5">&gt;_Branchat_&lt;</h1>
                <p class="text-lg text-slate-400 font-medium">基于Deepseek v4的分支交互式AIChat开源桌面应用</p>
            </div>

            <div v-else class="space-y-5">
                <div v-for="message in messages" :key="message.id" class="space-y-4">
                    <div v-if="message.role === 'user'" class="flex justify-end">
                        <div class="bg-indigo-600 text-white rounded-2xl rounded-br-md px-5 py-3 max-w-[70%] shadow-sm">
                            <div class="msg msg-user text-white select-text!">{{ message.content }}</div>
                        </div>
                    </div>
                    
                    <div v-else-if="message.role === 'assistant'" class="flex justify-start" @mouseenter="showSideToolsId = message.id" @mouseleave="showSideToolsId = null">
                        <div class="bg-white border border-slate-200 rounded-2xl rounded-bl-md px-5 py-3 max-w-[70%] shadow-sm">
                            <div class="msg msg-ai text-slate-700 **:select-text!" v-html="message.htmlContent || message.content"></div>
                        </div>
                        <span v-show="showSideToolsId === message.id" class="flex items-end ml-2 cursor-pointer gap-1 text-slate-300 hover:text-slate-400 transition-colors duration-200" @click="backtrace(message.id)">
                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="21" viewBox="0 5 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 10L9 4l-6 6"/><path d="M20 20h-7a4 4 0 0 1-4-4V5"/></svg>
                            回溯至这里
                        </span>
                    </div>
                </div>
            </div>
        </div>

        <Transition name="mask" enter-active-class="transition ease-in-out duration-300" enter-from-class="opacity-0"
            enter-to-class="opacity-100" leave-active-class="transition ease-in-out duration-300"
            leave-from-class="opacity-100" leave-to-class="opacity-0">
            <div v-if="showHistory || showConfig" class="fixed inset-0 bg-black/40 z-40" @click="panelClose"></div>
        </Transition>
        <History :isVisible="showHistory" @close="panelClose" @load="loadHistoryToApp"
            @delete="deleteHistoryItem" @export="exportHistoryItem" @import="importHistoryItem"
            @new-conversation="createNewConversation" @update-title="handleUpdateTitle" />
        <UserConfig :isVisible="showConfig" />

        <div v-show="!isGivenOptions"
            class="composer-bar fixed bottom-0 left-0 right-0 bg-indigo-50 backdrop-blur-sm border-t border-slate-200 flex items-center justify-between p-4 pb-8.5 pt-8 w-full shadow-[0_-4px_20px_rgba(0,0,0,0.06)]">
            <div class="composer-left w-3/4 flex space-x-3 ml-3">
                <button @click="showHistory = true, historyStore.loadHistoryItems()"
                    class="w-9 h-9 rounded-xl bg-white shadow-sm hover:shadow-md border border-slate-200 flex items-center justify-center self-center transition-all duration-200 hover:border-indigo-300 hover:bg-indigo-50 group cursor-pointer">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none"
                        stroke="#64748b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="group-hover:stroke-indigo-500 transition-colors">
                        <circle cx="12" cy="12" r="10"></circle>
                        <polyline points="12 6 12 12 16 14"></polyline>
                    </svg>
                </button>
            </div>

            <div class="composer-center absolute left-1/2 transform -translate-x-1/2 flex space-x-3">
                <input type="text" placeholder="输入您的问题/指令..." @keydown="textareaEnter"
                    class="composer-input w-lg px-4 py-3 border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-400 focus:border-transparent transition-all duration-200 bg-slate-50 placeholder-slate-400" />

                <div class="flex space-x-3">
                    <button @click="sendMsg" :disabled="isSending"
                        :class="isSending ? 'bg-slate-300 cursor-not-allowed' : 'bg-indigo-600 hover:bg-indigo-700 shadow-sm hover:shadow-md'"
                        class="text-white px-5 py-3 rounded-xl transition-all duration-200 flex items-center justify-center whitespace-nowrap font-medium cursor-pointer">
                        <svg v-if="!isSending" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path>
                        </svg>
                        <span v-else>发送中...</span>
                    </button>
                </div>
            </div>

            <div class="composer-right w-3/4 flex justify-end mr-3">
                <button @click="showConfig = true"
                    class="w-9 h-9 rounded-xl bg-white shadow-sm hover:shadow-md border border-slate-200 flex items-center justify-center self-center transition-all duration-200 hover:border-indigo-300 hover:bg-indigo-50 group cursor-pointer">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none"
                        stroke="#64748b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="group-hover:stroke-indigo-500 transition-colors">
                        <circle cx="12" cy="12" r="3"></circle>
                        <path
                            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z">
                        </path>
                    </svg>
                </button>
            </div>
        </div>

        <OptionSelection 
            v-if="isGivenOptions"
            :raw="options.raw"
            :positive="options.positive"
            :positiveExtraInput="options.positiveExtraInput"
            :negative="options.negative"
            :negativeExtraInput="options.negativeExtraInput"
            @select="handleOptionSelection"
            @close="handleOptionClose"
        />

        <Notification :visible="notificationVisible" :message="notificationMessage" :mode="notificationMode" @close="closeNotification" @confirm="onNotificationConfirm" @cancel="onNotificationCancel" />
        <Snackbar :message="snackbarMessage" :visible="snackbarVisible" @close="closeSnackbar" />
    </div>
</template>

<style>
@import 'katex/dist/katex.min.css';

.msg-ai {
    line-height: 1.75;
    font-size: 0.94rem;
}

.msg-ai h1 {
    font-size: 1.5em;
    font-weight: 700;
    margin: 0.6em 0 0.3em;
    color: #0f172a;
    letter-spacing: -0.01em;
}

.msg-ai h2 {
    font-size: 1.25em;
    font-weight: 600;
    margin: 0.5em 0 0.25em;
    color: #1e293b;
}

.msg-ai h3 {
    font-size: 1.1em;
    font-weight: 600;
    margin: 0.4em 0 0.2em;
    color: #334155;
}

.msg-ai hr {
    border: none;
    height: 1px;
    background: linear-gradient(to right, transparent, #cbd5e1, transparent);
    margin: 1em 0;
}

.msg-ai p {
    margin: 0.4em 0;
}

.msg-ai pre {
    background: #1e293b;
    padding: 16px 20px;
    border-radius: 10px;
    overflow: auto;
    margin: 0.6em 0;
    border: 1px solid #334155;
}

.msg-ai pre code {
    background: transparent;
    color: #e2e8f0;
    padding: 0;
    border-radius: 0;
    font-family: 'Consolas', 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Monaco', monospace;
    font-size: 0.85em;
    line-height: 1.6;
}

.msg-ai code {
    font-family: 'Consolas', 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Monaco', monospace;
    background-color: #f1f5f9;
    color: #6366f1;
    padding: 0.15em 0.4em;
    border-radius: 4px;
    font-size: 0.88em;
    font-weight: 500;
}

.msg-ai blockquote {
    border-left: 3px solid #6366f1;
    color: #475569;
    background: #f8fafc;
    padding: 0.5em 1em;
    margin: 0.5em 0;
    border-radius: 0 6px 6px 0;
}

.msg-ai strong {
    color: #0f172a;
    font-weight: 600;
}

.msg-ai table {
    border-collapse: collapse;
    width: 100%;
    margin: 0.6em 0;
    font-size: 0.9em;
    border-radius: 8px;
    overflow: hidden;
}

.msg-ai th,
.msg-ai td {
    border: 1px solid #e2e8f0;
    padding: 8px 14px;
    text-align: left;
}

.msg-ai th {
    background: #f1f5f9;
    color: #334155;
    font-weight: 600;
}

.msg-ai tr {
    background-color: #fff;
}

.msg-ai tr:nth-child(2n) {
    background-color: #f8fafc;
}

.msg-ai a {
    color: #6366f1;
    text-decoration: none;
    border-bottom: 1px solid rgba(99, 102, 241, 0.3);
    transition: border-color 0.15s ease;
}

.msg-ai a:hover {
    border-bottom-color: #6366f1;
}

.msg-ai ul,
.msg-ai ol {
    padding-left: 1.5em;
    margin: 0.4em 0;
}

.msg-ai li {
    margin: 0.15em 0;
}

.msg-ai li::marker {
    color: #94a3b8;
}

.msg-ai img {
    max-width: 100%;
    border-radius: 8px;
}

.app-shell {
    height: calc(100dvh - 32px - 102px - env(safe-area-inset-top, 0px));
}

.composer-bar {
    padding-left: calc(1rem + env(safe-area-inset-left, 0px));
    padding-right: calc(1rem + env(safe-area-inset-right, 0px));
    padding-bottom: calc(2.125rem + env(safe-area-inset-bottom, 0px));
}

@media (max-width: 768px) {
    .app-shell {
        height: calc(100dvh - 72px - env(safe-area-inset-top, 0px));
    }

    .composer-bar {
        gap: 0.75rem;
        padding-top: 0.875rem;
        padding-bottom: calc(0.875rem + env(safe-area-inset-bottom, 0px));
    }

    .composer-left,
    .composer-right {
        width: auto;
        margin: 0;
        flex: 0 0 auto;
    }

    .composer-center {
        position: static;
        transform: none;
        flex: 1 1 auto;
        min-width: 0;
    }

    .composer-input {
        width: 100%;
        min-width: 0;
    }
}
</style>