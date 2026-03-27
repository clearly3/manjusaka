<template>
  <PageWrapper dense contentFullHeight contentBackground fixedHeight>
    <div class="flex flex-col h-full">
    <CollapseContainer>
        <template #title>
        <div v-if="state.agent.ntype === 'npc1'">
        <PopConfirmButton
          :disabled="state.agent.ntype !== 'npc1'"
          title="确定销毁该NPC吗?"
          type="danger"
          @confirm="NpcExit"
          >销毁NPC</PopConfirmButton
        >
        <Upload
          :show-upload-list="false"
          :before-upload="beforeUpload"
          :multiple="true"
          :disabled="state.agent.ntype !== 'npc1'"
        >
        <a-button type="primary" :disabled="state.agent.ntype !== 'npc1'">上传文件</a-button>
        </Upload>
        <a-button
          @click="Screen"
          type="primary"
          :disabled="state.agent.ntype !== 'npc1'"
        >
          一键截屏
        </a-button>
        <a-button type="ghost" />
        <PopConfirmButton
          type="danger"
          :title="'确定' + (state.agent.npc2 ? '卸载' : '加载') + 'NPC2吗?'"
          @confirm="Npc2Load"
        >{{ state.agent.npc2 ? "卸载" : "加载" }}NPC2</PopConfirmButton>
        </div>
        
        <PopConfirmButton
          :disabled="!state.agent.npc2"
          title="确定打开文件管理吗?"
          type="primary"
          @confirm="OpenFile(state.agent.id)"
        >文件管理</PopConfirmButton>

        <PopConfirmButton
          :disabled="!state.agent.npc2"
          title="确定打开虚拟终端吗?"
          type="primary"
          @confirm="OpenCmd(state.agent.id)"
        >虚拟终端</PopConfirmButton>

        <PopConfirmButton
          :disabled="!state.agent.npc2"
          title="确定打开虚拟桌面吗?"
          type="primary"
          @confirm="OpenVnc(state.agent.id)"
        >虚拟桌面</PopConfirmButton>
        <a-button
          @click="CreateProxy"
          type="primary"
          :disabled="!state.agent.npc2"
        >新建隧道</a-button>
        <a-button
          @click="OpenAiChat(state.agent.id)"
          type="primary"
          :disabled="!state.agent.npc2"
        >AI聊天</a-button>
        <a-button type="ghost" />
        <a-button v-if="state.agent.ntype === 'npc1'" @click="RunPlugin" type="primary"> 运行插件 </a-button>
        </template>
        <Description @register="agentinfo" :data="state.agent" />
      </CollapseContainer>
      <template v-if="state.agent.ntype === 'npc1'">
      <div class="mt-2 flex flex-grow-0">
        <a-input
          :disabled="state.agent.ntype !== 'npc1'"
          v-model:value="state.cmd"
          @pressEnter="CmdSend"
          placeholder="输入help查看帮助"
          autocomplete="on"
        >
        <template #addonBefore>输入命令： </template>
        </a-input>
        <a-button
          type="danger"
          @click="CmdSend"
          :disabled="state.agent.ntype !== 'npc1'"
          >发送</a-button
        >
      </div>
      </template>
      <div class="terminal-body flex-1 min-h-0">
        <div ref="terminalRef" class="terminal" />
      </div>
    </div>
  </PageWrapper>
  <ProxyModal @register="registerModalproxy" />
  <PlugModal @register="registerModalplug" @success="PlugSend"/>
</template>

<script lang="ts">
import "@xterm/xterm/css/xterm.css";
import {
  defineComponent,
  reactive,
  nextTick,
  onMounted,
  onUnmounted,
  watchEffect,
  watch,
  h,
  ref,
  toRaw,
} from "vue";
import { Input, InputNumber, Tag, Select, Upload } from "ant-design-vue";
import { CollapseContainer } from "/@/components/Container/index";
import { PageWrapper } from "/@/components/Page";
import { PopConfirmButton } from "/@/components/Button";
import { Time } from "/@/components/Time";
import { BasicForm, FormSchema, useForm } from "/@/components/Form/index";
import {
  Description,
  DescItem,
  useDescription,
} from "/@/components/Description/index";
import { useModal } from "/@/components/Modal";
import { formatToDateTime, formatToDate, dateUtil } from "/@/utils/dateUtil";
import { useMessage } from "/@/hooks/web/useMessage";
import { useTabs } from "/@/hooks/web/useTabs";
import { useGo } from "/@/hooks/web/usePage";
import { createImgPreview } from "/@/components/Preview/index";
import { ITheme, Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { downloadByData } from "/@/utils/file/download";
import { copyTextToClipboard } from "/@/hooks/web/useCopyToClipboard";
import { nps } from "./../nps.js";
import { PostApi } from "/@/api";
import ProxyModal from "./../../proxys/RoleModal.vue";
import PlugModal from "./../PlugModal.vue";


export default defineComponent({
  name: "AgentInfo",
  components: {
    PageWrapper,
    CollapseContainer,
    BasicForm,
    Description,
    InputNumber,
    Upload,
    PopConfirmButton,
    Time,
    Terminal,
    FitAddon,
    ProxyModal,
    PlugModal
  },
  props: ["agent", "event"],
  emits: ["onsend"],
  setup(props, { emit }) {
    const terminalRef: any = ref(null);
    const state = reactive({
      agent: {},
      cmd: "",
      pwd: "",
      history: [],
      crtlist: [],
    });
    const go = useGo();
    const [registerModalplug, { openModal: openModalplug,closeModal: closeModalplug }] = useModal();
    const [registerModalproxy, { openModal: openModalproxy,closeModal: closeModalproxy }] = useModal();
    const { createMessage } = useMessage();
    const [agentinfo] = useDescription({
      column: 2,
      schema: [
        {
          field: "id",
          label: "NPCID",
        },
        {
          field: "pid",
          label: "PID",
        },
        {
          field: "platform",
          label: "系统",
        },
        {
          field: "arch",
          label: "构架",
        },
        {
          field: "hostname",
          label: "主机名",
        },
        {
          field: "intranet",
          label: "内网地址",
        },
        {
          field: "username",
          label: "用户名",
        },
        {
          field: "internet",
          label: "公网地址",
        },
        {
          field: "target",
          label: "路由",
        },
        {
          field: "note",
          label: "备注",
        },
        {
          field: "ntype",
          label: "NPC类型",
        },
        {
          field: "updateat",
          label: "最后回连",
          render: (value, record) => {
            return formatToDateTime(record.updateat * 1000);
          },
        },
        {
          field: "process",
          label: "进程路径",
        },
      ],
    });
    const fitAddon = new FitAddon();

    const term = new Terminal({
      fontSize: 15, // 稍微调小字体
      fontWeight: "normal",
      fontFamily: "Consolas",
      rendererType: "canvas", //渲染类型
      cursorBlink: false, // 只读模式下不显示光标
      disableStdin: false, // 禁用输入
      allowProposedApi: true,
      fastScrollModifier: "ctrl",
      scrollback: 1000000,
      theme: {
        background: "#002833", // 背景色
        cursor: "#268F81", // 设置光标
      } as ITheme,
    });
    term.onKey((e) => {
      if (e.domEvent.ctrlKey || e.domEvent.metaKey) {
        e.domEvent.preventDefault();
        if (e.domEvent.key === "c") {
          copyTextToClipboard(term.getSelection());
        }
        if (e.domEvent.key === "s") {
          const buffer = term.buffer.active;
          let content = "";
          for (let i = 0; i < buffer.length; i++) {
            const line = buffer.getLine(i);
            if (line) {
              content += line.translateToString(true) + "\n";
            }
          }
          downloadByData(content, state.agent.intranet + ".log");
        }
      }
    });

    const bytesToSize = (bytes: number) => {
      const units = ["Bytes", "KB", "MB", "GB", "TB"];
      if (bytes == 0) return "-";
      const i = Math.floor(Math.log(bytes) / Math.log(1024));
      const size = (bytes / 1024 ** i).toFixed(0);
      if (i === 0) {
        return "1 KB";
      }
      return `${size} ${units[i]}`;
    };

    const utf8ArrayToStr = (array) => {
      let utf8decoder = new TextDecoder(); // default 'utf-8' or 'utf8'
      return utf8decoder.decode(array).replace(/\r\n|\r|\n/g, "\r\n");
    };

    function getBase64(file: File) {
      return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.readAsDataURL(file);
        reader.onload = () => resolve(reader.result);
        reader.onerror = (error) => reject(error);
      });
    }

    function Screen() {
      let action = nps.Action.create();
      action.act = "cr";
      let event = nps.AgentEvent.create();
      event.action = action;
      emit("onsend", event);
      createMessage.success(`截屏命令已发送！ 请等待返回。`);
    }

    function NpcExit() {
      let action = nps.Config.create();
      action.action = 2;
      let event = nps.AgentEvent.create();
      event.conf = action;
      emit("onsend", event);
      createMessage.success(`NPC销毁命令发送`);
    }

    function OpenCmd(id) {
      go("/agentcmd/" + id);
    }

    function OpenFile(id) {
      go("/agentfile/" + id);
    }

    function OpenVnc(id) {
      go("/agentvnc/" + id);
    }

    function OpenRdp(id) {
      go("/agentrdp/" + id);
    }

    function OpenAiChat(id) {
      go("/agentaichat/" + id);
    }
    

    function Npc2Load() {
      if (!state.agent.npc2) {
        let plugin = nps.Plugin.create();
        plugin.act = "ex";
        plugin.name = "npc2";
        let event = nps.AgentEvent.create();
        event.plugin = plugin;
        emit("onsend", event);
        createMessage.success(`NPC2加载中。。。 请等待返回！`);
      } else {
        PostApi({ action: "npc2stop", data: { id: state.agent.id } })
          .then((data) => {
            state.agent.npc2 = false;
          })
          .catch((e) => {
            //createMessage.error(`读取目录失败 ${e}`);
          });
      }
    }

    function CreateProxy() {
      openModalproxy(true, {
        record: {
          id: state.agent.id, // 将ID传递给表单
        },
      });
    }

    function RunPlugin() {
      openModalplug(true, {
        record: {
          id: state.agent.id, // 将ID传递给表单
        },
      });
    }

    function PlugSend(event) {
      emit("onsend", event);
      closeModalplug();
    }

    const beforeUpload = (file: File) => {
      const reader = new FileReader();
      reader.readAsArrayBuffer(file);
      reader.onload = (e) => {
        if (e.target?.result instanceof ArrayBuffer) {
          const data = new Uint8Array(e.target.result);
          let action = nps.Action.create();
          action.act = "fw";
          action.path = file.name;
          action.data = data;
          let event = nps.AgentEvent.create();
          event.action = action;
          emit("onsend", event);
          createMessage.success(
            `文件:${file.name} 大小:${bytesToSize(
              file.size
            )} 上传中... 请等待返回。`
          );
        }
      };
      reader.onerror = () => {
        createMessage.error("文件读取失败");
      };
      return false;
    };

    function CmdSend() {
      let cmds = state.cmd.split(" ");
      let action = nps.Action.create();
      let entrys = [];
      let entry = nps.Entry.create();
      switch (cmds[0]) {
        case "ss":
          action.act = "ss";
          break;
        case "ps":
          action.act = "ps";
          break;
        case "cr":
          action.act = "cr";
          break;
        case "cd":
          action.act = "cd";
          action.path = cmds[1];
          break;
        case "ls":
          action.act = "ls";
          action.path = cmds[1];
          break;
        case "fr":
          action.act = "fr";
          action.path = cmds[1];
          break;
        case "fw":
          const binaryString = atob(cmds[2]);
          const bytes = new Uint8Array(binaryString.length);
          for (let i = 0; i < binaryString.length; i++) {
            bytes[i] = binaryString.charCodeAt(i);
          }
          action.act = "fw";
          action.path = cmds[1];
          action.data = bytes;
          break;
        case "sh":
          action.act = "sh";
          action.name = btoa(cmds.slice(1).join(" "));
          break;
        case "pl":
          action.act = "pl";
          action.name = cmds[1];
          action.args = cmds.slice(2).join(" ");
          break;
        default:
          if (state.cmd == "?" || state.cmd == "help") {
            term.writeln("");
            term.writeln("help/cls  帮助/清屏 ctrl+s 保存会话记录");
            term.writeln("sh        执行命令 sh cmd");
            term.writeln("ls        枚举文件 ls path");
            term.writeln("cd        切换目录 cd path");
            term.writeln("ps        查看进程 ps");
            term.writeln("ss        查看网络 ss");
            term.writeln("cr        执行截屏 cr");
            term.writeln("fr        读取文件 fr path");
            term.writeln("fw        写入文件 fw path base64");
            term.writeln("pl        执行插件 pl name <args>");
            term.write(
              `[${state.agent.username}@${state.agent.hostname} ${state.pwd}]#`
            );
          }
          if (state.cmd == "cls") {
            term.reset();
            term.write(
              `[${state.agent.username}@${state.agent.hostname} ${state.pwd}]#`
            );
          }
          state.cmd = "";
          return;
      }
      let event = nps.AgentEvent.create();
      event.action = action;
      emit("onsend", event);
      term.writeln(state.cmd);
      createMessage.success(`命令:${state.cmd} 已发送！ 请等待返回。`);
      state.history.push(state.cmd);
      state.cmd = "";
    }

    onMounted(() => {
      term.open(terminalRef.value);
      term.loadAddon(fitAddon);
      fitAddon.fit(); // 初始适配
      const resizeObserver = new ResizeObserver(() => {
        fitAddon.fit();
      });
      resizeObserver.observe(terminalRef.value);
      term.reset();
    });

    onUnmounted(() => {
      term.dispose();
    });

    watchEffect(() => {
      if (props.agent) {
        if (state.agent.id != props.agent.id) {
          state.agent = props.agent;
          let path = props.agent.process.replaceAll("\\", "/").split("/");
          path.pop();
          state.pwd = path.join("/");
          term.write(
            `[${state.agent.username}@${state.agent.hostname} ${state.pwd}]#`
          );
        }
      }

      if (props.event) {
        if (props.event.plugin) {
          if (
            props.event.plugin.act === "ss" &&
            props.event.plugin.name === "npc2"
          ) {
            setTimeout(() => {
              state.agent.npc2 = true;
            }, 5000); // 3秒
          }
          if (props.event.plugin.act === "dd") {
            term.writeln(props.event.plugin.name,props.event.plugin.args);
            term.writeln(props.event.plugin.data);
            term.write(
              `[${state.agent.username}@${state.agent.hostname} ${state.pwd}]#`
            );
          }
        }
        if (props.event.action) {
          term.writeln("");
          switch (props.event.action.act) {
            case "cd":
              state.pwd = props.event.reqcd.path.replaceAll("\\", "/");
              term.writeln(props.event.action.path);
              break;
            case "ls":
              //term.writeln("ls " + props.event.reqls.path);
              props.event.action.entrys.map((entry) => {
                const icon = entry.isfile ? "📄" : "📁";
                term.writeln(
                  `${icon}   ${entry.path.padEnd(88)}  ${
                    formatToDateTime(entry.modified * 1000)?.padEnd(20) ||
                    "".padEnd(20)
                  } ${bytesToSize(entry.size)}`
                );
              });
              break;
            case "fw":
              term.writeln(`文件写入成功！ 文件名:${props.event.action.path} `);
              break;
            case "fr":
              if (props.event.action.data.length <= 4096) {
                term.writeln(utf8ArrayToStr(props.event.action.data));
              } else {
                term.writeln(
                  `文件读取成功！ 请前往敏感文件项下载文件, 文件大小 ${bytesToSize(
                    props.event.action.data.length
                  )} `
                );
              }
              break;
            case "sh":
              term.writeln(utf8ArrayToStr(props.event.action.data));
              break;
            case "ss":
              term.writeln(utf8ArrayToStr(props.event.action.data));
              break;
            case "ps":
              term.writeln(utf8ArrayToStr(props.event.action.data));
              break;
            case "pl":
              term.writeln(utf8ArrayToStr(props.event.action.data));
              break;
            case "cr": //如果npc一直主动推送图片过来可能会造成npu界面假死
              /*let array = props.event.action.data;
              var binary = "";
              for (var len = array.byteLength, i = 0; i < len; i++) {
                binary += String.fromCharCode(array[i]);
              }
              let url = "data:image/png;base64," + window.btoa(binary);
              if (state.crtlist.indexOf(url) == -1) {
                state.crtlist.push(url);
              }
              createImgPreview({
                imageList: state.crtlist,
                defaultWidth: 1000,
                scaleStep: 10,
                index: state.crtlist.indexOf(url),
              });*/
              term.writeln(
                `屏幕截图成功！ 请前往敏感文件项查看截图, 文件大小 ${bytesToSize(
                  props.event.action.data.length
                )} 。`
              );
              break;
            case "err":
              term.writeln(
                `出错了！ 错误 ${utf8ArrayToStr(props.event.action.data)}`
              );
              break;
            default:
          }
          term.write(
            `[${state.agent.username}@${state.agent.hostname} ${state.pwd}]#`
          );
        }
      }
    });

    return {
      state,
      agentinfo,
      terminalRef,
      CmdSend,
      Screen,
      beforeUpload,
      NpcExit,
      Npc2Load,
      registerModalproxy,
      registerModalplug,
      CreateProxy,
      RunPlugin,
      PlugSend,
      OpenCmd,
      OpenFile,
      OpenVnc,
      OpenRdp,
      OpenAiChat,
    };
  },
});
</script>

<style lang="less" scoped>
.terminal-body {
  width: 99.9%;
  height: 99.9%;
}

.terminal {
  width: 100%;
  height: 100%;
  .xterm .xterm-viewport {
    overflow-y: hidden;
  }
}

</style>
