<template>
  <PageWrapper dense contentFullHeight contentBackground fixedHeight>
    <div id="chat-container" ref="chatContainerRef" class="chat-container">
      <div class="chat-toolbar">
        <div class="toolbar-left">
          <span class="chat-title">AI聊天窗口 - {{state.agent.username}}@{{state.agent.intranet}}</span>
        </div>
      </div>

      <!-- 消息列表区域 -->
      <div class="message-list-wrapper" ref="messageListWrapperRef">
        <div class="message-list" ref="messageListRef">
          <div
            v-for="message in messages"
            :key="message.id"
            class="message-item"
            :class="{ 'message-self': message.isMine }"
          >
            <div class="message-avatar">
              <div class="avatar" :class="{ 'avatar-self': message.isMine }">
                {{ message.isMine ? '我' : state.agent.username }}
              </div>
            </div>
            <div class="message-content-wrapper">
              <div class="message-bubble" :class="{ 'bubble-self': message.isMine }">
                <div class="message-text">{{ message.text }}</div>
              </div>
              <div class="message-time">{{ formatTime(message.timestamp) }}</div>
            </div>
          </div>
          <div v-if="loading" class="loading-tip">发送中...</div>
        </div>
      </div>

      <!-- 底部输入区域 -->
      <div class="input-area">
        <a-textarea
          v-model:value="inputText"
          placeholder="请输入消息..."
          :auto-size="{ minRows: 2, maxRows: 6 }"
          @pressEnter="handlePressEnter"
          :disabled="sending"
          class="message-input"
        />
        <a-button
          type="primary"
          @click="sendMessage"
          :loading="sending"
          :disabled="!inputText.trim()"
          class="send-btn"
        >
          发送
        </a-button>
      </div>
    </div>
  </PageWrapper>
</template>

<script lang="ts">
import {
  defineComponent,
  onMounted,
  onBeforeUnmount,
  ref,
  unref,
  reactive,
  computed,
  nextTick,
} from "vue";

import { PageWrapper } from "/@/components/Page";
import { useGlobSetting } from "/@/hooks/setting";
import { useRouter } from "vue-router";
import { useTabs } from "/@/hooks/web/useTabs";
import { PostApi } from "/@/api";
import axios from 'axios';

// 消息数据类型
interface Message {
  id: string;
  text: string;
  timestamp: number;
  isMine: boolean;
}

export default defineComponent({
  name: "AgentAiChat",
  components: { PageWrapper },
  setup() {
    const state = reactive({
      agent: {},
    });
    const { setTitle } = useTabs();
    const { currentRoute } = useRouter();

    const query = computed(() => unref(currentRoute).params);

    const { apiUrl = "/manjusaka" } = useGlobSetting();

    // DOM 引用
    const chatContainerRef = ref<HTMLDivElement | null>(null);
    const messageListWrapperRef = ref<HTMLDivElement | null>(null);
    const messageListRef = ref<HTMLDivElement | null>(null);

    // 聊天数据
    const messages = ref<Message[]>([]);
    const inputText = ref("");
    const sending = ref(false);
    const loading = ref(false);

    // 生成唯一 ID
    const generateId = () => `${Date.now()}-${Math.random().toString(36).substr(2, 8)}`;

    // 格式化时间
    const formatTime = (timestamp: number) => {
      const date = new Date(timestamp);
      const hours = date.getHours().toString().padStart(2, "0");
      const minutes = date.getMinutes().toString().padStart(2, "0");
      return `${date}-${hours}:${minutes}`;
    };

    // 滚动到底部
    const scrollToBottom = () => {
      nextTick(() => {
        if (messageListWrapperRef.value) {
          messageListWrapperRef.value.scrollTop = messageListWrapperRef.value.scrollHeight;
        }
      });
    };

    // 添加消息
    const addMessage = (text: string, isMine: boolean) => {
      const newMessage: Message = {
        id: generateId(),
        text,
        timestamp: Date.now(),
        isMine,
      };
      messages.value.push(newMessage);
      scrollToBottom();
      return newMessage;
    };

    const sendMessage = async () => {
      const content = inputText.value.trim();
      if (!content || sending.value) return;
      addMessage(content, true);
      inputText.value = "";

      try {
        const response = await axios.post(
          apiUrl + '/agentaichat',
          { id: unref(query).id, data: content },
          { timeout: 0 }
        );
        addMessage(response.data.result, false);
      } catch (error) {
        if (error.response) {
          const status = error.response.status;
          const data = error.response.data;
          addMessage(`${status}: ${data}`, false);
        }
      }
    };

    // 处理回车发送
    const handlePressEnter = (e: KeyboardEvent) => {
      if (!e.shiftKey) {
        e.preventDefault();
        sendMessage();
      }
    };

    const adjustMessageListHeight = () => {
      scrollToBottom();
    };

    const handleResize = () => {
      adjustMessageListHeight();
    };
    function OnInit() {
      PostApi({
        action: "agentinfo",
        data: { id: unref(query).id },
      }).then((e) => {
          state.agent = e;
          setTitle("AI - " + state.agent.username + "@" + state.agent.intranet);
      }).catch(() => {
          //createMessage.error("初始化失败");
      });
    }

    onMounted(async () => {
      OnInit();
      scrollToBottom();
      window.addEventListener("resize", handleResize);
    });

    onBeforeUnmount(() => {
      window.removeEventListener("resize", handleResize);
    });

    return {
      chatContainerRef,
      messageListWrapperRef,
      messageListRef,
      messages,
      inputText,
      sending,
      loading,
      formatTime,
      sendMessage,
      handlePressEnter,
      state,
    };
  },
});
</script>

<style lang="less" scoped>
.chat-container {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background-color: #f5f5f5;
  overflow: hidden;
  position: relative;
}

/* 顶部工具栏 */
.chat-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 16px;
  background-color: #fff;
  border-bottom: 1px solid #e8e8e8;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
  z-index: 10;
  flex-shrink: 0;
}

.toolbar-left {
  .chat-title {
    font-size: 16px;
    font-weight: 500;
    color: #333;
  }
}

/* 消息列表区域（可滚动） */
.message-list-wrapper {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
  background-color: #f5f5f5;
  scroll-behavior: smooth;
}

.message-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 100%;
}

/* 消息项 */
.message-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  max-width: 80%;
  
  &.message-self {
    flex-direction: row-reverse;
    margin-left: auto;
    
    .message-content-wrapper {
      align-items: flex-end;
    }
    
    .message-bubble {
      background-color: #95ec69;
      color: #000;
      border-radius: 8px 2px 8px 8px;
    }
    
    .message-time {
      text-align: right;
    }
  }
}

.message-avatar {
  flex-shrink: 0;
  
  .avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background-color: #c4c4c4;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    color: #fff;
    font-weight: 500;
    
    &.avatar-self {
      background-color: #576b95;
    }
  }
}

.message-content-wrapper {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-width: calc(100% - 52px);
}

.message-bubble {
  background-color: #fff;
  padding: 8px 12px;
  border-radius: 2px 8px 8px 8px;
  box-shadow: 0 1px 1px rgba(0, 0, 0, 0.1);
  word-break: break-word;
  max-width: 100%;
  
  .message-text {
    font-size: 14px;
    line-height: 1.5;
    white-space: pre-wrap;
  }
}

.message-time {
  font-size: 11px;
  color: #999;
  padding: 0 4px;
}

.loading-tip {
  text-align: center;
  font-size: 12px;
  color: #999;
  padding: 8px;
}

/* 底部输入区域 */
.input-area {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  padding: 12px 16px;
  background-color: #fff;
  border-top: 1px solid #e8e8e8;
  flex-shrink: 0;
  
  .message-input {
    flex: 1;
    border-radius: 4px;
    
    :deep(textarea) {
      min-height: 100px;
      resize: vertical;
    }
  }
  
  .send-btn {
    height: 36px;
    min-width: 70px;
    border-radius: 4px;
  }
}

/* 滚动条样式 */
.message-list-wrapper::-webkit-scrollbar {
  width: 6px;
}

.message-list-wrapper::-webkit-scrollbar-track {
  background: #f0f0f0;
  border-radius: 3px;
}

.message-list-wrapper::-webkit-scrollbar-thumb {
  background: #c1c1c1;
  border-radius: 3px;
  
  &:hover {
    background: #a8a8a8;
  }
}
</style>