<template>
  <div>
    <BasicTable @register="registerTableproj">
      <template #toolbar>
        <a-button type="primary" @click="handleAgentCreate"> 生成NPC1 </a-button>
        <a-button type="primary" @click="handleCreate"> 新建项目 </a-button>
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
import { useDrawer } from "/@/components/Drawer";
import { useUserStore } from "/@/store/modules/user";
import { useModal } from "/@/components/Modal";
import RoleDrawer from "./RoleDrawer.vue";
import RoleModal from "./RoleModal.vue";
import { formatToDate } from "/@/utils/dateUtil";

export default defineComponent({
  name: "ProjectManagement",
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
    const [registerDrawer, { openDrawer }] = useDrawer();
    const [registerModal, { openModal }] = useModal();

    const [registerTableproj, { reload }] = useTable({
      title: "NPC1监听器",
      columns: [
        {
          title: "项目名称",
          dataIndex: "name",
        },
        {
          title: "回调地址",
          dataIndex: "callback1",
        },
        {
          title: "路由",
          dataIndex: "route",
        },
        {
          title: "密钥",
          dataIndex: "enckey",
        },
        {
          title: "当前",
          width: 100,
          customRender: ({ record }) => {
            return h(Switch, {
              checked: record.id == userStore.getTarget && !(false == (state.target = record)), //
              checkedChildren: "当前",
              unCheckedChildren: "选择",
              onChange(checked: boolean) {
                PostApi({ action: "projectself", data: { id: record.id } })
                  .then(() => {
                    userStore.setTarget(record.id);
                    state.target = record;
                    createMessage.success(`当前项目: ${record.name}`);
                  })
                  .catch(() => {
                    //createMessage.error("修改当前项目失败");
                  })
                  .finally(() => {});
              },
            });
          },
        },
        {
          title: "状态",
          dataIndex: "isrun",
          width: 100,
          customRender: ({ record }) => {
            return h(Switch, {
              checked: record.isrun == true,
              checkedChildren: "已启用",
              unCheckedChildren: "已禁用",
              onChange(e: boolean) {
                record.isrun = e;
                PostApi({ action: "projectupdate", data: record })
                  .then(() => {
                    record.isrun = e;
                    createMessage.success(`已成功修改项目状态`);
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
        title: "操作",
        dataIndex: "action",
        slots: { customRender: "action" },
        fixed: undefined,
      },
      beforeFetch: function (params) {
        const { page, pageSize } = params;
        return { action: "projectlist", data:{page: page, size: pageSize }};
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
      PostApi({ action: "projectdelete", data: { id: record.id } })
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
      registerTableproj,
      registerDrawer,
      registerModal,
      handleCreate,
      handleEdit,
      handleDelete,
      handleSuccess,
      handleAgentCreate,
    };
  },
});
</script>
