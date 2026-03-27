<template>
  <div>
    <BasicTable @register="registerTablelisten">
      <template #toolbar>
        <!-- <a-button type="primary" @click="handleAgentCreate"> 生成NPC2 </a-button> -->
        <a-button type="primary" @click="handleCreate"> 新建监听器 </a-button>
      </template>
      <template #action="{ record }">
        <TableAction
          :actions="[
            {
              icon: 'clarity:note-edit-line',
              label: '编辑',
              onClick: handleEdit.bind(null, record),
            },
            {
              icon: 'ant-design:delete-outlined',
              label: '删除',
              color: 'error',
              popConfirm: {
                title: '是否确认删除',
                confirm: handleDelete.bind(null, record),
              },
            },
          ]"
        />
      </template>
    </BasicTable>
    <RoleDrawer @register="registerDrawer" @success="handleSuccess" />
    <RoleModal @register="registerModal" />
  </div>
</template>
<script lang="ts">
import { defineComponent, h, reactive } from "vue";
import { Switch, Radio } from "ant-design-vue";
import { BasicTable, useTable, TableAction } from "/@/components/Table";
import { BasicForm, useForm } from "/@/components/Form/index";
import { Description, DescItem, useDescription } from "/@/components/Description/index";
import { PostApi } from "/@/api";
import { useMessage } from "/@/hooks/web/useMessage";
import { useModal } from "/@/components/Modal";
import { useDrawer } from "/@/components/Drawer";
import { useUserStore } from "/@/store/modules/user";
import RoleDrawer from "./RoleDrawer.vue";
import RoleModal from "./RoleModal.vue";
import { formatToDate } from "/@/utils/dateUtil";

export default defineComponent({
  name: "ListenManagement",
  components: {
    BasicTable,
    RoleModal,
    RoleDrawer,
    TableAction,
    Description,
    BasicForm,
  },
  setup() {
    const state = reactive({
      target: {},
    });
    const { createMessage } = useMessage();
    const userStore = useUserStore();
    const [registerModal, { openModal }] = useModal();
    const [registerDrawer, { openDrawer }] = useDrawer();
    const [registerTablelisten, { reload }] = useTable({
      title: "NPC2监听器",
      columns: [
        {
          title: "协议",
          dataIndex: "transport",
          width: 120,
        },
        {
          title: "监听地址",
          dataIndex: "listenaddr",
        },
        {
          title: "上线地址",
          dataIndex: "onlineaddr",
        },
        {
          title: "密钥",
          dataIndex: "enckey",
        },
        {
          title: "加密方式",
          dataIndex: "noise",
        },
        {
          title: "状态",
          dataIndex: "isrun",
          width: 120,
          customRender: ({ record }) => {
            return h(Switch, {
              checked: record.isrun == true,
              checkedChildren: "已启用",
              unCheckedChildren: "已禁用",
              onChange(e: boolean) {
                record.isrun = e;
                PostApi({ action: "listenaction", data: {id:record.id, isrun:record.isrun}})
                  .then(() => {
                    record.isrun = e;
                    createMessage.success(`已成功修改监听器状态`);
                  })
                  .catch(() => {
                    //createMessage.error("修改项目状态失败");
                  })
                  .finally(() => {});
              },
            });
          },
        },
        {
          title: "更新时间",
          dataIndex: "updateat",
          width: 100,
          customRender: ({ record }) => {
            return formatToDate(new Date(record.updateat * 1000));
          },
        },
      ],
      api: PostApi,
      showTableSetting: true,
      bordered: true,
      showIndexColumn: false,
      actionColumn: {
        width: 150,
        title: "操作",
        dataIndex: "action",
        slots: { customRender: "action" },
        fixed: undefined,
      },
      beforeFetch: function (params) {
        const { page, pageSize } = params;
        return { action: "listenlist", data:{page: page, size: pageSize }};
      },
    });

    function handleCreate() {
      openDrawer(true, {
        isUpdate: false,
      });
      reload();
    }

    function handleEdit(record: Recordable) {
      openDrawer(true, {
        record,
        isUpdate: true,
      });
      reload();
    }

    function handleDelete(record: Recordable) {
      PostApi({ action: "listendelete", data: { id: record.id } })
        .then(() => {
          createMessage.success("删除成功");
        })
        .catch(() => {
          //createMessage.error("删除失败");
        })
        .finally(() => {
          reload();
        });
    }

    function handleSuccess() {
      reload();
    }

    function handleAgentCreate(record: Recordable) {
      let data = {
        name: state.target.name,
        callback1: state.target.callback1,
        route: state.target.route,
        platform: "1",
        arch: "1",
      };
      openModal(true, data);
    }

    return {
      registerTablelisten,
      registerModal,
      registerDrawer,
      handleCreate,
      handleEdit,
      handleDelete,
      handleSuccess,
      handleAgentCreate
    };
  },
});
</script>
