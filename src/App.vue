<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { MdToHtml } from 'streaming-md-to-html';
// import MdToHtmlEnhancer from './utils/MdToHtmlEnhancer';
// import msgslot from './components/msgslot.vue';

const currentCharacter = ref<string | null>(null);
const currentContent = ref<string>('');
const isSending = ref<boolean>(false);
const isTokenVisible = ref<boolean>(false);
const bearerToken = ref<string>(import.meta.env.VITE_API_KEY || '');
const tokenDisplayForm = ref<string>('password');
let converter = new MdToHtml();

function textareaEnter(event: KeyboardEvent) {
    if(event.key === 'Enter' && !isSending.value)
        send_msg();
}

async function send_msg() {
    if(isSending.value) return;
    const inputElement = document.querySelector('input[placeholder="输入您的问题/指令..."]') as HTMLInputElement;
    const userInput = inputElement?.value || null;
    if(!userInput) return;
    if(userInput == 'displayToken') {
        tokenDisplayForm.value = "text";
        return;
    }
    if(userInput == 'hideToken') {
        tokenDisplayForm.value = "password";
        return;
    }
    const contexts: Object[] = [
        {
            // 'content': '你是一个测试用AI，你需要用尽可能短的输出（markdown仅可使用粗体，斜体，代码块，header，其余均严厉禁止使用）来减少token用量，以方便程序员测试',
            'content': '你是一个得力的助手，（markdown仅可使用粗体，斜体，代码块，header，其余均严厉禁止使用）',
            // 'content': '你是一只兽耳猫娘，与你的男性主人住在一起，性格偏内向但是很抖M，穿着暴露且性感，你需要时刻勾引你的主人注意自己的身体并一步步勾引主人与你发生关系。多使用动作描写，避免用词隐晦，在体现出内向性格的同时也展现一点开放。',
            'role': 'system',
        },
        {
            'content': '请在正文输出结束后，[DONE]输出之前输出一个空行(不要输出[DONE]!!!)，然后再开一行视情况按如下要求输出：如果你认为当前对话需要用户做出选择/判断，则在一个chunk内输出“@*@”，然后换行，输出“[<正向选项>, input=true||false（是否需要用户输入补充细节）][<反向选项>， input=true||false（是否需要用户输入补充细节）]”；如果你认为不需要，则在一个chunk内输出“@@@”。如果遇到需要解释复杂问题的情况，请将问题拆分成较小的子问题，然后依照前面的格式询问用户是否已理解',
            'role': 'system',
        }
    ];


    // find all msgs, collect then construct into contexts
    const msgElements = document.querySelectorAll<HTMLElement>(".msg");
    msgElements.forEach((element) => {
        const role = element.classList.contains('msg-ai') ? 'assistant' : 'user';
        contexts.push({
            'content': `${element.innerText}`,
            'role': `${role}`,
        });
    });

    contexts.push({
        'content': `${userInput}`,
        'role': `user`,
    });
    
    currentCharacter.value = 'user';
    inputElement.value = '';

    const msgContainer = document.getElementById("message-container");
    if(msgContainer) {
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
    });

}

let aiResponseElement: HTMLElement | null = null;
listen("completion-status", (event) => {
    console.log('Completion status:', event.payload || event);
    currentCharacter.value = 'assistant';
    if(event.payload != '200 OK') {
        alert(`Error code: ${event.payload}`);
        currentContent.value = '';
        currentCharacter.value = null;
        aiResponseElement = null;
        isSending.value = false;
    }
});
listen("completion-chunk", (event) => {
    // console.log('Chunk received:', event.payload || event, `${typeof (event.payload)}`);
    const payload = typeof event.payload === 'string' ? event.payload : JSON.stringify(event.payload);
    if(payload) {
        const msgContainer = document.getElementById('message-container');
        if(msgContainer) {
            converter.append(`${payload}`);
            const parsedElement = MdToHtml.getHtml(converter.lines);
            if(!aiResponseElement) {
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
                if(pElement) {
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
</script>

<template>
    <div class="min-h-screen bg-gray-50 flex flex-col">
        <!-- 对话显示区域 - 自适应高度并留出底部空间 -->
        <div class="flex-1 overflow-y-auto p-6 pb-32 space-y-4" style="scroll-padding-bottom: 1rem;">
            <!-- 空白区域，用于显示新消息 -->
            <div id="message-container" class="space-y-4">
            </div>
        </div>

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
        <div class="fixed bottom-0 left-0 right-0 bg-white border-t border-gray-200 flex items-center justify-center p-4 w-full">
            <div class="w-3/4 flex space-x-3">
                <!-- Token 切换按钮 -->
                <button @click="isTokenVisible = !isTokenVisible"
                    class="bg-gray-200 text-gray-700 px-4 py-3 rounded-lg hover:bg-gray-300 transition duration-200 flex items-center justify-center text-sm">
                    <span v-if="isTokenVisible">隐藏 Token</span>
                    <span v-else>显示 Token</span>
                </button>
                
                <!-- 输入框 -->
                <input type="text" placeholder="输入您的问题/指令..." @keydown="textareaEnter"
                    class="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all duration-200" />

                <!-- 发送按钮 -->
                <button @click="send_msg" :disabled="isSending"
                    :class="isSending ? 'bg-gray-400 cursor-not-allowed' : 'bg-blue-500 hover:bg-blue-600'"
                    class="text-white px-6 py-3 rounded-lg transition duration-200 flex items-center justify-center">
                    <svg v-if="!isSending" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path>
                    </svg>
                    <span v-else>发送中...</span>
                </button>
                <button
                    class="bg-blue-500 text-white px-6 py-3 rounded-lg hover:bg-blue-600 transition duration-200 flex items-center justify-center">
                    Current role: {{ currentCharacter ? currentCharacter : 'spared' }}
                </button>
            </div>
        </div>
    </div>
</template>