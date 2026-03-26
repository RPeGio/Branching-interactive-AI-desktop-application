<script setup lang="ts">
import type { HistoryItem } from '../data/types'

const props = defineProps<{
    isVisible: boolean;
    historyItems: HistoryItem[];
}>();
const emit = defineEmits<{ 
    close: [];
    load: [item: HistoryItem];
    'new-conversation': [];
}>();


const closeHistory = () => {
    emit('close');
};

// 新建对话
const createNewConversation = () => {
    // 触发新建对话事件
    emit('new-conversation');
    closeHistory(); // 创建新对话后关闭历史面板
};

// 加载选中的历史记录
const loadHistoryToApp = (item: HistoryItem) => {
    emit('load', item);
    closeHistory(); // 加载后关闭面板
};
</script>

<template>
    <Teleport to="body">
        <Transition name="history-panel" enter-active-class="transition ease-in-out duration-300"
            enter-from-class="transform -translate-x-full opacity-0"
            enter-to-class="transform translate-x-0 opacity-100"
            leave-active-class="transition ease-in-out duration-300"
            leave-from-class="transform translate-x-0 opacity-100"
            leave-to-class="transform -translate-x-full opacity-0">
            <div class="fixed top-0 left-0 h-full w-2/5 bg-[#eefbff] shadow-xl z-50 p-6 overflow-y-auto mt-7.5" v-if="props.isVisible" @click.stop>
                <div class="flex justify-between items-center mb-6">
                    <h2 class="text-xl font-bold">历史对话记录</h2>
                    <div class="flex space-x-3">
                        <button @click="createNewConversation" class="text-gray-500 hover:text-gray-700 text-xl">
                            <!-- 新建对话图标 -->
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="16"></line><line x1="8" y1="12" x2="16" y2="12"></line></svg>
                        </button>
                        <button @click="closeHistory" class="w-5 h-5 text-gray-500 hover:text-gray-700 text-2xl">
                            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                        </button>
                    </div>
                </div>

                <!-- 历史记录列表 -->
                <div class="space-y-2">
                    <div v-for="item in [...props.historyItems].reverse()" :key="item.id"
                        class="w-full h-12 rounded-lg bg-[#ffffff] flex items-center px-4 cursor-pointer shadow-sm hover:scale-105 hover:shadow-lg transition-all duration-200"
                        @click="loadHistoryToApp(item)">
                        <span class="truncate w-7/10">{{ item.title }}</span>
                        <span class="w-3/10 flex justify-end items-center text-gray-400 text-sm">{{ item.date }}</span>
                    </div>
                </div>
            </div>
        </Transition>
    </Teleport>
</template>