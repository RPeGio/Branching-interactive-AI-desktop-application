<script setup lang="ts">
const props = defineProps<{
    isVisible: boolean;
    historyItems: HistoryItem[];
}>();
const emit = defineEmits<{
    close: [];
    load: [item: HistoryItem];
}>();

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

const closeHistory = () => {
    emit('close');
};

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
                    <button @click="closeHistory" class="text-gray-500 hover:text-gray-700 text-2xl">
                        &times;
                    </button>
                </div>

                <!-- 历史记录列表 -->
                <div class="space-y-3">
                    <div v-for="(item, index) in props.historyItems" :key="index"
                        class="w-full h-12 rounded-lg bg-[#ffffff] flex items-center px-4 cursor-pointer shadow-sm hover:scale-105 hover:shadow-lg transition-all duration-200"
                        @click="loadHistoryToApp(item)">
                        <span class="truncate">{{ item.title }}</span>
                    </div>
                </div>
            </div>
        </Transition>
    </Teleport>
</template>