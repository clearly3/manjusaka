<template>
  <div>
    <BasicTable @register="registerTablesets">
      <template #toolbar>
        <a-button type="primary" @click="ChangePwd"> 修改密码 </a-button>
      </template>
      <template #action="{ record }">
        <TableAction
          :actions="[
            {
              icon: 'ant-design:delete-outlined',
              color: 'error',
              label: '删除',
              popConfirm: {
                title: '确认删除?',
                confirm: handleDelete.bind(null, record),
              },
            },
          ]"
        />
      </template>
    </BasicTable>
    <RoleModal @register="registerModal" />
  </div>
</template>
<script lang="ts">
import { defineComponent } from "vue";

import { BasicTable, useTable, TableAction } from "/@/components/Table";
import { formatToDate, formatToDateTime } from "/@/utils/dateUtil";
import { useModal } from "/@/components/Modal";
import { useMessage } from "/@/hooks/web/useMessage";
import { PostApi } from "/@/api";
import RoleModal from "./RoleModal.vue";

export default defineComponent({
  name: "RoleManagement",
  components: { BasicTable, TableAction, RoleModal },
  setup() {
    const { createMessage } = useMessage();
    const [registerModal, { openModal }] = useModal();
    const [registerTablesets, { reload }] = useTable({
      title: "全局设置选项",
      columns: [
        {
          title: "键",
          dataIndex: "key",
          align: "right",
        },
        {
          title: "值",
          dataIndex: "value",
          align: "left",
          edit: true,
        },
        {
          title: "备注",
          width: 250,
          align: "left",
          dataIndex: "valuenote",
        },
        {
          title: "时间",
          width: 200,
          dataIndex: "updateat",
          customRender: ({ record }) => {
            return formatToDate(new Date(record.updateat * 1000));
          },
        },
      ],
      showTableSetting: true,
      bordered: true,
      showIndexColumn: false,
      size: "small",
      beforeEditSubmit: handleEdit,
      clickToRowSelect: false,
      api: PostApi,
      pagination: {
        pageSizeOptions: [100, 200, 500, 1000],
        pageSize: 100,
      },
      actionColumn: {
        width: 150,
        title: "操作",
        dataIndex: "action",
        slots: { customRender: "action" },
        fixed: undefined,
      },
      beforeFetch: function (params) {
        const { page, pageSize } = params;
        return { action: "settinglist", pagenum: page, pagesize: pageSize };
      },
    });

    function handleEdit(record: Recordable) {
      console.log(record);
      PostApi({ action: "settingupdate", data: {
          key:record.record.key, 
          value:record.value,
        } 
      }).then(() => {
          createMessage.success("更新成功");
        })
        .catch(() => {
          //createMessage.error("更新失败");
        })
        .finally(() => {
          reload();
        });
    }

    function handleDelete(record: Recordable) {
      PostApi({ action: "settingdelete", data: { id: record.key } })
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

    function ChangePwd() {
      openModal(true, {});
    }


    return {
      registerTablesets,
      registerModal,
      handleEdit,
      handleDelete,
      ChangePwd
    };
  },
});
</script>
