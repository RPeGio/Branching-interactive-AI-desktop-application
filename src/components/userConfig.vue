<script setup lang="ts">
import { reactive } from 'vue';

interface ConfigItem {
    globalSystemPrompt: string;
    currentSystemPrompt: string;
    temperature: number;
    maxTokens: number;
    topP: number;
    frequencyPenalty: number;
}

interface Props {
    isVisible: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
    close: [];
}>();

// 用户配置数据
const config = reactive<ConfigItem>({
    globalSystemPrompt: '你是一个得力的助手，（markdown仅可使用粗体，斜体，代码块，header，其余均严厉禁止使用）',
    currentSystemPrompt: '',
    temperature: 1,
    maxTokens: 4000,
    topP: 0.9,
    frequencyPenalty: 0.5
});

// 关闭配置面板
const closeConfig = () => {
    emit('close');
};

defineExpose({
    config
});
</script>

<template>
    <!-- # TODO: 自定义用户配置
    1. 自定义system prompt/Global system prompt，分别用两个栏目实现，每个栏目设有属性名称，名称旁的显示帮助icon，下方占全宽的文本框
    2. 自定义模型参数，用滑条实现
    3. 指令列表指南 -->
    <Teleport to="body">
        <Transition name="config-panel" enter-active-class="transition ease-in-out duration-300"
            enter-from-class="transform translate-x-full opacity-0" enter-to-class="transform translate-x-0 opacity-100"
            leave-active-class="transition ease-in-out duration-300"
            leave-from-class="transform translate-x-0 opacity-100"
            leave-to-class="transform translate-x-full opacity-0">
            <div class="fixed top-0 right-0 h-full w-2/5 bg-[#eefbff] shadow-xl z-50 p-6 overflow-y-auto mt-7.5"
                v-if="props.isVisible" @click.stop>
                <div class="flex justify-between items-center mb-6">
                    <h2 class="text-xl font-bold">用户配置</h2>
                    <button @click="closeConfig" class="text-gray-500 hover:text-gray-700 text-2xl">
                        &times;
                    </button>
                </div>

                <!-- 自定义SystemPrompt -->
                <div class="mb-8">
                    <h3 class="text-lg font-semibold mb-4 flex items-center">
                        自定义系统提示词
                    </h3>

                    <!-- 全局系统提示 -->
                    <div class="mb-6">
                        <div class="flex items-center mb-2">
                            <label class="font-medium">全局系统提示词</label>
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 ml-2 text-gray-500"
                                fill="none" viewBox="0 0 24 24" stroke="currentColor" title="全局系统提示适用于所有对话">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                            </svg>
                        </div>
                        <textarea v-model="config.globalSystemPrompt"
                            class="w-full h-32 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                            placeholder="输入全局系统提示词..."></textarea>
                    </div>

                    <!-- 当前对话系统提示 -->
                    <div>
                        <div class="flex items-center mb-2">
                            <label class="font-medium">当前对话系统提示词</label>
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 ml-2 text-gray-500"
                                fill="none" viewBox="0 0 24 24" stroke="currentColor" title="当前对话系统提示仅适用于当前对话">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                            </svg>
                        </div>
                        <textarea v-model="config.currentSystemPrompt"
                            class="w-full h-32 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                            :placeholder="config.globalSystemPrompt"></textarea>
                    </div>
                </div>

                <!-- 自定义模型参数 -->
                <div class="mb-8">
                    <h3 class="text-lg font-semibold mb-4">自定义模型参数</h3>

                    <div class="space-y-4">
                        <div>
                            <div class="flex justify-between mb-1">
                                <span>temperature</span>
                                <span>{{ config.temperature.toFixed(2) }}</span>
                            </div>
                            <input type="range" min="0" max="2" step="0.01" v-model.number="config.temperature"
                                class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer" />
                            <p class="text-sm text-gray-500 mt-1">控制AI回复的随机性，值越高越随机</p>
                        </div>

                        <div>
                            <div class="flex justify-between mb-1">
                                <span>max_tokens</span>
                                <span>{{ config.maxTokens }}</span>
                            </div>
                            <input type="range" min="500" max="8000" step="100" v-model.number="config.maxTokens"
                                class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer" />
                            <p class="text-sm text-gray-500 mt-1">控制AI回复的最大长度</p>
                        </div>

                        <div>
                            <div class="flex justify-between mb-1">
                                <span>top_p</span>
                                <span>{{ config.topP.toFixed(2) }}</span>
                            </div>
                            <input type="range" min="-1" max="1" step="0.01" v-model.number="config.topP"
                                class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer" />
                            <p class="text-sm text-gray-500 mt-1">temperature的替代方案（不建议同时修改top_p与temperature）</p>
                        </div>

                        <div>
                            <div class="flex justify-between mb-1">
                                <span>frequency_penalty</span>
                                <span>{{ config.frequencyPenalty.toFixed(2) }}</span>
                            </div>
                            <input type="range" min="-2" max="2" step="0.1" v-model.number="config.frequencyPenalty"
                                class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer" />
                            <p class="text-sm text-gray-500 mt-1">如果该值为正，将降低模型重复相同内容的可能性</p>
                        </div>
                    </div>
                </div>

                <!-- 指令列表指南 -->
                <div class="mb-6">
                    <h3 class="text-lg font-semibold mb-4">指令列表指南</h3>
                    <div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
                        <ul class="list-disc pl-5 space-y-2 text-sm">
                            <li><strong>/displayToken</strong>: 显示当前 API Token</li>
                            <li><strong>/hideToken</strong>: 隐藏当前 API Token</li>
                            <li><strong>/balance</strong>: 查询当前API Token余额</li>
                            <li><strong>/clearHistory</strong>: 清空所有历史对话记录</li>
                        </ul>
                    </div>
                </div>
            </div>
        </Transition>
    </Teleport>
</template>