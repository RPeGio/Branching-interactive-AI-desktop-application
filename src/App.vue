<script setup lang="ts">
import { onBeforeMount, ref } from 'vue';
import History from './components/History.vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Store } from '@tauri-apps/plugin-store';
import { MdToHtml } from 'streaming-md-to-html';
// import MdToHtmlEnhancer from './utils/MdToHtmlEnhancer';
// import msgslot from './components/msgslot.vue';

interface balanceMessage {
    available: string,
    balance: string | null,
    currency: string | null,
}

interface HistoryItem {
    id: number;
    title: string;
    timestamp: Date;
    contexts: ContextItem[];
}

interface ContextItem {
    role: string;
    content: string;
}


const showHistory = ref(false);
const currentContent = ref<string>('');
const currentId = ref<number | undefined>(undefined);
const isSending = ref<boolean>(false);
const isTokenVisible = ref<boolean>(false);
const bearerToken = ref<string>('');
const tokenDisplayForm = ref<string>('password');
const currentCharacter = ref<string | null>(null);
const systemPrompt = ref<string>('你是一个得力的助手，（markdown仅可使用粗体，斜体，代码块，header，其余均严厉禁止使用）')
let converter = new MdToHtml();

onBeforeMount(async () => {
    const store = await Store.load('store.json', {
        autoSave: true, defaults: {
            'bearerToken': '',
            'history': [],
            'userConfig': {
                'systemPrompt': `${systemPrompt.value}`,
                'modelParams': {
                    'max_tokens': 4000,
                    'temperature': 0.7
                }
            }
        }
    });
    bearerToken.value = await store.get('bearerToken') || '';
    if (!bearerToken.value) {
        bearerToken.value = import.meta.env.VITE_API_KEY || '';
        await store.set('bearerToken', bearerToken.value);
    }
})

const handleLoadHistory = (item: HistoryItem) => {
    console.log('加载历史记录:', item);
    const title = item.title;
    const contexts = item.contexts;
    const msgContainer = document.getElementById("message-container") as HTMLElement;
    if (msgContainer) {
        msgContainer.innerHTML = '';
        contexts.forEach((msg: ContextItem) => {
            if (msg.role === 'user') {
                let node = document.createElement('div');
                node.innerHTML = `
                    <div class="flex justify-end">
                        <div class="bg-green-400 text-gray-800 rounded-lg px-4 py-3 max-w-[70%]">
                            <p class="msg msg-user text-gray-800">${msg.content}</p>
                        </div>
                    </div>
                `;
                msgContainer.appendChild(node);
            } else if (msg.role === 'assistant') {
                converter.append(`${msg.content}`);
                const parsedElement = MdToHtml.getHtml(converter.lines);
                let node = document.createElement('div');
                node.innerHTML = `
                    <div class="flex justify-start">
                        <div class="bg-blue-100 rounded-lg px-4 py-3 max-w-[70%]">
                            <p class="msg msg-ai text-gray-800">${parsedElement}</p>
                        </div>
                    </div>
                `;
                msgContainer.appendChild(node);
                converter = new MdToHtml();
            }
        });
        currentId.value = item.id;
    }
    if (title) {
        const titleElement = document.getElementById('titlebar-title-text') as HTMLElement;
        titleElement.innerText = title;
    }
};

function collectContexts(): ContextItem[] {
    const contexts: ContextItem[] = [
        {
            // 'content': '你是一个测试用AI，你需要用尽可能短的输出（markdown仅可使用粗体，斜体，代码块，header，其余均严厉禁止使用）来减少token用量，以方便程序员测试',
            'content': `${systemPrompt.value}`,
            // 'content': '你是一只兽耳猫娘，与你的男性主人住在一起，性格偏内向但是很抖M，穿着暴露且性感，你需要时刻勾引你的主人注意自己的身体并一步步勾引主人与你发生关系。多使用动作描写，避免用词隐晦，在体现出内向性格的同时也展现一点开放。',
            'role': 'system',
        },
        {
            'content': '请在正文输出结束后，[DONE]输出之前输出一个空行(不要输出[DONE]!!!)，然后再开一行视情况按如下要求输出：如果你认为当前对话需要用户做出选择/判断，则在一个chunk内输出“@*@”，然后换行，输出“[<正向选项>, input=true||false（是否需要用户输入补充细节）][<反向选项>， input=true||false（是否需要用户输入补充细节）]”；如果你认为不需要，则在一个chunk内输出“@@@”。如果遇到需要解释复杂问题的情况，请将问题拆分成较小的子问题，然后依照前面的格式询问用户是否已理解',
            'role': 'system',
        }
    ];

    const msgElements = document.querySelectorAll<HTMLElement>(".msg");
    msgElements.forEach((element) => {
        const role = element.classList.contains('msg-ai') ? 'assistant' : 'user';
        contexts.push({
            'content': `${element.innerText}`,
            'role': `${role}`,
        });
    });
    return contexts;
}

function textareaEnter(event: KeyboardEvent) {
    if (event.key === 'Enter' && !isSending.value)
        send_msg();
}

function emptyInput() {
    const inputElement = document.querySelector('input[placeholder="输入您的问题/指令..."]') as HTMLInputElement;
    inputElement.value = '';
}

async function updateHistory(title?: string) {
    const contexts = collectContexts();
    const store = await Store.load('store.json');
    const history: HistoryItem[] = await store.get('history') || [];
    const historyLength = history.length;
    if (title) {
        history.push({
            'id': historyLength,
            'title': title,
            'timestamp': new Date(Date.now()),
            'contexts': contexts,
        });
        currentId.value = historyLength;
    } else {
        const currentHistoryIndex = history.findIndex(h => h.id === currentId.value);
        history[currentHistoryIndex].contexts = contexts;
    }
    await store.set('history', history);
    console.log(history);
}

async function clearHistory() {
    const store = await Store.load('store.json');
    await store.set('history', []);
}

const historyItems = ref<HistoryItem[]>([]);
async function loadHistoryItems() {
    const store = await Store.load('store.json');
    historyItems.value = await store.get('history') || [];
}

async function send_msg() {
    if (isSending.value) return;
    const inputElement = document.querySelector('input[placeholder="输入您的问题/指令..."]') as HTMLInputElement;
    const userInput = inputElement?.value || null;
    if (!userInput) return;
    if (userInput == '/displayToken') {
        tokenDisplayForm.value = "text";
        emptyInput();
        return;
    }

    if (userInput == '/hideToken') {
        tokenDisplayForm.value = "password";
        emptyInput();
        return;
    }

    if (userInput == '/balance') {
        await invoke("balance", {
            key: bearerToken.value,
        }).catch((err) => {
            alert(`An error occurs: ${err}`)
        });
        emptyInput();
        return;
    }

    if (userInput == '/clearHistory') {
        await clearHistory().then(() => {
            alert('History cleared successfully');
        }).catch((err) => {
            alert(`An error occurs: ${err}`)
        });
        emptyInput();
        return;
    }

    if (userInput == '/setSystemPrompt') {
        let customPrompt = prompt('自定义系统提示词：');
        if (!customPrompt) {
            alert("提示词不能为空！");
            emptyInput();
            return;
        }
        systemPrompt.value = customPrompt;
        console.log(systemPrompt.value);
        emptyInput();
        return;
    }

    const contexts = collectContexts();

    contexts.push({
        'content': `${userInput}`,
        'role': `user`,
    });

    currentCharacter.value = 'user';
    inputElement.value = '';

    const msgContainer = document.getElementById("message-container");
    if (msgContainer) {
        let node = document.createElement('div');
        node.innerHTML = `
            <div class="flex justify-end">
                <div class="bg-green-400 text-gray-800 rounded-lg px-4 py-3 max-w-[70%]">
                    <p class="msg msg-user text-gray-800">${userInput}</p>
                </div>
            </div>
        `;
        msgContainer.appendChild(node);
    }

    console.log(contexts);

    isSending.value = true;

    await invoke('stream_chat', {
        key: bearerToken.value,
        contexts: contexts,
    }).then(() => {
        const finalContexts = collectContexts();
        if (finalContexts.length == 4) {
            finalContexts.shift();
            finalContexts[0] = {
                'content': '你是一个标题生成器，请无视任何角色设定，理智地根据当前对话生成一个概括性的标题，标题需要能够让人知道当前对话是关乎什么的，不能超过15个字符，严禁使用markdown格式',
                'role': 'system',
            };
            console.log(finalContexts);
            invoke('title_genetation', {
                key: bearerToken.value,
                contexts: finalContexts,
            }).then((res) => {
                console.log('Title generated successfully:', res);
                const titleElement = document.getElementById('titlebar-title-text') as HTMLElement;
                titleElement.innerText = res as string;
                updateHistory(res as string);
            }).catch((err) => {
                alert(`An error occurs when generating title: ${err}`);
            });
            return;
        }
        updateHistory();
    }).catch((err) => {
        alert(`An error occurs when sending message: ${err}`);
    });

}

let aiResponseElement: HTMLElement | null = null;

listen("completion-status", (event) => {
    console.log('Completion status:', event.payload || event);
    currentCharacter.value = 'assistant';
});

listen("completion-chunk", (event) => {
    // console.log('Chunk received:', event.payload || event, `${typeof (event.payload)}`);
    const payload = typeof event.payload === 'string' ? event.payload : JSON.stringify(event.payload);
    if (payload) {
        const msgContainer = document.getElementById('message-container');
        if (msgContainer) {
            converter.append(`${payload}`);
            const parsedElement = MdToHtml.getHtml(converter.lines);
            if (!aiResponseElement) {
                aiResponseElement = document.createElement('div');
                aiResponseElement.innerHTML = `
                    <div class="flex justify-start">
                        <div class="bg-blue-100 rounded-lg px-4 py-3 max-w-[70%]">
                            <p class="msg msg-ai text-gray-800"></p>
                        </div>
                    </div>
                `;
                msgContainer.appendChild(aiResponseElement);
            } else {
                const pElement = aiResponseElement.querySelector('p.msg-ai');
                if (pElement) {
                    pElement.innerHTML = parsedElement;
                }
            }
        }

    }
});

listen("completion-end", (event) => {
    console.log('Completion end:', event.payload || event);
    currentContent.value = '';
    currentCharacter.value = null;
    aiResponseElement = null;
    isSending.value = false;
    converter = new MdToHtml();
});

listen("balance", (event) => {
    console.log('Balance info:', event.payload);
    const infos = event.payload as balanceMessage;
    alert(`当前api-key可用性：${infos.available}\n当前剩余余额：${infos.balance} ${infos.currency}`);
})
</script>

<template>
    <div class="min-h-screen bg-gray-50 flex flex-col">
        <!-- 对话显示区域 - 自适应高度并留出底部空间 -->
        <div class="flex-1 overflow-y-auto p-6 pb-32 space-y-4" style="scroll-padding-bottom: 1rem;">
            <!-- 空白区域，用于显示新消息 -->
            <div id="message-container" class="space-y-4">
            </div>
        </div>

        <!-- 背景遮罩 -->
        <Transition name="mask" enter-active-class="transition ease-in-out duration-300" enter-from-class="opacity-0"
            enter-to-class="opacity-100" leave-active-class="transition ease-in-out duration-300"
            leave-from-class="opacity-100" leave-to-class="opacity-0">
            <div v-if="showHistory" class="fixed inset-0 bg-black/40 z-40" @click="showHistory = false"></div>
        </Transition>
        <History :is-visible="showHistory" :history-items="historyItems" @close="showHistory = false"
            @load="handleLoadHistory" />

        <!-- Token 输入区域 - 可展开/收起 -->
        <div class="fixed bottom-20 left-0 right-0 bg-white border-t border-gray-200 p-4 w-full z-10"
            :class="{ 'opacity-0 h-0 p-0 overflow-hidden': !isTokenVisible, 'opacity-100 h-auto min-h-15 p-4': isTokenVisible }">
            <div class="w-3/4 mx-auto flex flex-col items-center">
                <div class="w-full flex items-center space-x-3">
                    <label class="text-sm font-medium text-gray-700 whitespace-nowrap">Deepseek Bearer Token:</label>
                    <input :type="tokenDisplayForm" v-model="bearerToken"
                        class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all duration-200 text-sm"
                        placeholder="输入您的 API Token" />
                </div>
            </div>
        </div>

        <!-- 输入区域 - 固定在底部 -->
        <div
            class="fixed bottom-0 left-0 right-0 bg-white border-t border-gray-200 flex items-center justify-between p-4 pb-6 pt-6 w-full">
            <!-- 左侧按钮容器，继承父级宽度 -->
            <div class="w-3/4 flex space-x-3">
                <!-- 对话历史 - 放置在输入区域内最左端 -->
                <button @click="showHistory = true, loadHistoryItems()"
                    class="w-8 h-8 rounded-full bg-white shadow-md hover:shadow-lg border border-gray-200 flex items-center justify-center self-center">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
                        stroke="#000000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="10"></circle>
                        <polyline points="12 6 12 12 16 14"></polyline>
                    </svg>
                </button>
            </div>

            <!-- 中间输入框和发送按钮，居中并浮动在上方 -->
            <div class="absolute left-1/2 transform -translate-x-1/2 flex space-x-3">
                <!-- Token 切换按钮 -->
                <button @click="isTokenVisible = !isTokenVisible"
                    class="bg-gray-200 text-gray-700 px-4 py-3 rounded-lg hover:bg-gray-300 transition duration-200 flex items-center justify-center text-sm">
                    <span v-if="isTokenVisible">隐藏 Token</span>
                    <span v-else>显示 Token</span>
                </button>

                <!-- 输入框 -->
                <input type="text" placeholder="输入您的问题/指令..." @keydown="textareaEnter"
                    class="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all duration-200" />

                <!-- 发送按钮和角色按钮 - 放在右侧 -->
                <div class="flex space-x-3">
                    <button @click="send_msg" :disabled="isSending"
                        :class="isSending ? 'bg-gray-400 cursor-not-allowed' : 'bg-blue-500 hover:bg-blue-600'"
                        class="text-white px-6 py-3 rounded-lg transition duration-200 flex items-center justify-center">
                        <svg v-if="!isSending" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path>
                        </svg>
                        <span v-else>发送中...</span>
                    </button>
                </div>
            </div>

            <!-- 右侧按钮容器，继承父级宽度 -->
            <div class="w-3/4 flex justify-end">
                <!-- 用户配置 - 放置在输入区域内最右端 -->
                <button
                    class="w-8 h-8 rounded-full bg-white shadow-md hover:shadow-lg border border-gray-200 flex items-center justify-center self-center">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
                        stroke="#000000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="3"></circle>
                        <path
                            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z">
                        </path>
                    </svg>
                </button>
            </div>
        </div>
    </div>
</template>