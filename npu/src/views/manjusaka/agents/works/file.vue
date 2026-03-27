<template>
  <PageWrapper dense contentFullHeight contentBackground fixedHeight>
    <BasicTable
      ref="tableRef"
      @register="registerTable"
      @row-dbClick="FileReadDir3"
      :dataSource="state.entrys"
    >
      <template #toolbar>
        <Upload 
          :show-upload-list="false" 
          :custom-request="FileUpload"
        >
          <a-button type="primary">上传文件</a-button>
        </Upload>
        <a-button
          type="dashed"
          @click="FileReadDir2"
        >上级</a-button>
        <a-input v-model:value="state.current_dir" />
        <a-button type="dashed" @click="FileReadDir">加载</a-button>
      </template>
      <template #action="{ record }">
        <TableAction
          :actions="[
            {
              label: '下载' ,
              auth: record.file_type !== 'File',
              onClick: FileDownload.bind(null, record),
            }
          ]"
        />
      </template>
    </BasicTable>
    
    <BasicModal
      v-bind="deleteModalConfig"
      @register="registerDeleteModal"
      @ok="handleDeleteConfirm"
    >
      <p>确定删除 "<span style="color: red;">{{ deleteFileName }}</span>" 吗？</p>
      <p style="color: #ff4d4f; font-size: 12px;">此操作不可撤销</p>
    </BasicModal>
    

    <BasicModal
      v-bind="renameModalConfig"
      @register="registerRenameModal"
      @ok="handleRenameConfirm"
    >
      <a-input 
        v-model:value="renameValue" 
        placeholder="新文件名" 
        @keydown.enter="handleRenameConfirm"
        ref="renameInputRef"
      />
    </BasicModal>
  </PageWrapper>
</template>

<script lang="ts">
import { 
  defineComponent, 
  reactive, 
  ref, 
  unref,
  computed,
  onMounted, 
  onUnmounted,
  nextTick,
  h 
} from "vue";
import { Icon } from "/@/components/Icon";
import { PageWrapper } from "/@/components/Page";
import { PopConfirmButton } from "/@/components/Button";
import { BasicColumn, BasicTable, useTable, TableAction } from "/@/components/Table";
import { BasicTree, TreeItem } from "/@/components/Tree/index";
import { BasicUpload } from "/@/components/Upload";
import { Upload, message, Input, Button } from "ant-design-vue";
import { useMessage } from "/@/hooks/web/useMessage";
import { useGlobSetting } from "/@/hooks/setting";
import { formatToDateTime } from "/@/utils/dateUtil";
import { UploadApi, PostApi, GetApi } from "/@/api";
import { downloadByUrl } from "/@/utils/file/download";
import { useContextMenu } from '/@/hooks/web/useContextMenu';
import { useRouter } from 'vue-router';
import { useTabs } from "/@/hooks/web/useTabs";
import { BasicModal, useModal } from "/@/components/Modal";

export default defineComponent({
  name: "AgentFile",
  components: {
    BasicTree,
    BasicTable,
    PageWrapper,
    PopConfirmButton,
    TableAction,
    Icon,
    Upload,
    BasicModal,
    BasicUpload,
  },
  props: [],
  emits: [],
  setup(props, ctx) {
    const state = reactive({
      agent: {},
      current_dir: "./",
      entrys: [],
      uploadlist: [],
    });

    const tableRef = ref<any>(null);
    const renameInputRef = ref<any>(null);
    const renameValue = ref('');
    const currentRecord = ref<any>(null);
    const deleteRecord = ref<any>(null);
    const deleteFileName = ref('');
    
    const { currentRoute } = useRouter();
    const query = computed(() => unref(currentRoute).params);
    const { closeCurrent, setTitle } = useTabs();
    const { createMessage } = useMessage();
    const { apiUrl = "/manjusaka" } = useGlobSetting();
    const [createContextMenu, destroyContextMenu] = useContextMenu({authRemove:true});

    // 删除确认模态框配置
    const deleteModalConfig = ref({
      title: '确认删除',
      visible: false,
      okText: '确定删除',
      okType: 'danger',
      cancelText: '取消',
    });

    // 重命名模态框配置
    const renameModalConfig = ref({
      title: '重命名',
      visible: false,
    });

    const [registerDeleteModal, { openModal: openDeleteModal, closeModal: closeDeleteModal }] = useModal();
    const [registerRenameModal, { openModal: openRenameModal, closeModal: closeRenameModal }] = useModal();

    const [registerTable, { reload }] = useTable({
      pagination: false,
      showTableSetting: false,
      tableSetting: {
        redo: true,
        size: false,
        setting: false,
      },
      showIndexColumn: false,
      size: "small",
      bordered: true,
      columns: [
        {
          dataIndex: "file_type",
          align: "right",
          sorter: true,
          width: 40,
          customRender: ({ record }) => {
            let iconName = '';
            switch (record.file_type) {
                case 'File':
                    iconName = 'ion:document';  // 文件图标
                    break;
                case 'Dir':
                    iconName = 'bi:folder';  // 文件夹图标
                    break;
                case 'Symlink':  // 可能的值
                    iconName = 'mdi:link-variant';  // 链接图标
                    break;
                default:
                    iconName = 'mdi:file-question-outline';  // 未知类型图标
            }
            return h(Icon, {
                size: 20,
                icon: iconName, 
            })
          },
        },
        {
          title: "名称",
          sorter: true,
          align:'left',
          dataIndex: "file_name"
        },
        {
          title: "大小",
          dataIndex: "size",
          sorter: true,
          width: 100,
          customRender: ({ record }) => {
            return  bytesToSize(record.size)
          },
        },
        {
          title: "权限",
          dataIndex: "permissions",
          width: 100,
        },
        {
          title: "用户组",
          dataIndex: "user",
          width: 150,
          customRender: ({ record }) => {
            return `u-${record.user},g-${record.group}`
          },
        },
        {
          title: "修改时间",
          sorter: true,
          width: 150,
          dataIndex: "modified",
        }
      ],
      clickToRowSelect: true,
      bordered: true,
      striped: true,
    });

    function bytesToSize(bytes: number){
      const units = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
      if (bytes === 0) return '-';
      const i = Math.floor(Math.log(bytes) / Math.log(1024));
      const size = (bytes / 1024 ** i).toFixed(0);
      if (i === 0) {
        return '1 KB';
      }
      return `${size} ${units[i]}`;
    };

    function FileUpload(params){
      params.action = apiUrl + '/sftp/upload/' + state.agent.id;
      params.filename = state.current_dir + '/' + params.file.name;
      UploadApi(params)
        .then((data) => {
          FileReadDir();
        }).catch((e) => {
          //createMessage.error(`文件上传失败 ${e}`);
        });
    }

    function FileReadDir(){
      let dir = state.current_dir;
      let path = dir === '' ? '/' : dir;
      PostApi({ action: "sftp/read_dir/" + state.agent.id, data: {id:"", path:state.current_dir}})
        .then((data) => {
          state.current_dir = data.current_dir.replaceAll('\\','/').split('/').slice(1).join('/');
          state.entrys = data.entrys;
        }).catch((e) => {
          //createMessage.error(`读取目录失败 ${e}`);
        });
    }

    function FileReadDir2(){ //上级目录
      let dir = state.current_dir.split('/').slice(0,-1).join('/');
      let path = dir === '' ? '/' : dir;
      PostApi({ action: "sftp/read_dir/" + state.agent.id, data: {id:"", path:path}})
        .then((data) => {
          state.current_dir = data.current_dir.replaceAll('\\','/').split('/').slice(1).join('/');
          state.entrys = data.entrys;
        }).catch((e) => {
          //createMessage.error(`读取目录失败 ${e}`);
        });
    }

    function FileReadDir3(record){ //双击打开目录
      if (record.file_type !== 'File') {
        let path = state.current_dir === '' ? record.file_name : state.current_dir + '/' + record.file_name;
        PostApi({ action: "sftp/read_dir/" + state.agent.id, data: {id:"", path:path}})
          .then((data) => {
            state.current_dir = data.current_dir.replaceAll('\\','/').split('/').slice(1).join('/');
            state.entrys = data.entrys;
          }).catch((e) => {
            //createMessage.error(`读取目录失败 ${e}`);
          });
      }
    }

    function GetDriveList(){
      PostApi({ action: "sftp/read_dir/" + state.agent.id, data: {id:"", path:'/'}})
      .then((data) => {
        state.current_dir = data.current_dir;
        state.entrys = data.entrys;
      }).catch((e) => {
        //createMessage.error(`读取目录失败 ${e}`);
      });
    }

    async function renameFile(record: any, newName: string){
      const oldPath = `${state.current_dir}/${record.file_name}`;
      const newPath = `${state.current_dir}/${newName}`;
      try {
        await PostApi({
          action: `sftp/rename/${state.agent.id}`,
          data: { id: oldPath, path: newPath },
        });
        createMessage.success(`重命名成功：${newName}`);
        FileReadDir(); // 刷新列表
      } catch (e) {
        createMessage.error(`重命名失败：${e.message || e}`);
        throw e;
      }
    }

    function showRenameModal(record: any) {
      currentRecord.value = record;
      renameValue.value = record.file_name;
      openRenameModal(true);
      
      // 等待模态框渲染完成后聚焦输入框
      nextTick(() => {
        if (renameInputRef.value) {
          renameInputRef.value.focus();
        }
      });
    }

    async function handleRenameConfirm() {
      if (!renameValue.value.trim()) {
        createMessage.warning('名称不能为空');
        return;
      }
      if (renameValue.value === currentRecord.value.file_name) {
        closeRenameModal();
        return;
      }
      
      try {
        await renameFile(currentRecord.value, renameValue.value);
        closeRenameModal();
      } catch (e) {
        // 错误已在renameFile中处理
      }
    }

    async function deleteFile(record: any) {
      const path = `${state.current_dir}/${record.file_name}`;
      try {
        await PostApi({
          action: `sftp/remove/${state.agent.id}`,
          data: { id: record.file_type, path },
        });
        createMessage.success(`删除成功：${record.file_name}`);
        FileReadDir();
      } catch (e) {
        createMessage.error(`删除失败：${e.message || e}`);
      }
    }

    function confirmDelete(record: any) {
      deleteRecord.value = record;
      deleteFileName.value = `${state.current_dir}/${record.file_name}`;
      openDeleteModal(true);
    }

    async function handleDeleteConfirm() {
      if (deleteRecord.value) {
        await deleteFile(deleteRecord.value);
        closeDeleteModal();
      }
    }

    function ContextMenu(record: any, e: MouseEvent) {
      e.preventDefault();
      createContextMenu({
        event: e,
        items: [
          ...(record.file_type !== 'File'
            ? [
                {
                  label: '进入',
                  icon: 'ant-design:enter-outlined',
                  handler: () => FileReadDir3(record),
                },
              ]
            : []),
          {
            label: '重命名',
            icon: 'ant-design:edit-outlined',
            handler: () => showRenameModal(record),
          },
          {
            label: '删除',
            icon: 'ant-design:delete-outlined',
            danger: true,
            handler: () => confirmDelete(record),
          },
          {
            label: '下载',
            icon: 'ant-design:download-outlined',
            disabled: record.file_type === 'Dir' || record.file_type === 'Symlink',
            handler: () => FileDownload(record),
          },
        ],
      });
    }

    function bindContextMenu() {
      // 等待DOM更新完成
      setTimeout(() => {
        const tableElement = document.querySelector('.ant-table-wrapper');
        if (tableElement) {
          tableElement.addEventListener('contextmenu', (e) => {
            let target = e.target as HTMLElement;
            while (target && !target.classList.contains('ant-table-row')) {
              target = target.parentElement!;
            }
            
            if (target) {
              const rowIndex = Array.from(target.parentElement!.children).indexOf(target);
              if (rowIndex >= 0 && state.entrys[rowIndex]) {
                const record = state.entrys[rowIndex];
                e.preventDefault();
                ContextMenu(record, e);
              }
            } else {
              // 如果没有点击到行上，阻止默认右键菜单
              e.preventDefault();
              // 不显示上下文菜单，因为点击的是空白区域
            }
          }, true);
          
        }
      }, 100);
    }

    function FileDownload(e){
      let path = state.current_dir + '/' + e.file_name;
      downloadByUrl({url: apiUrl + "/sftp/download?id="+state.agent.id + "&path=" + path, fileName: e.file_name});
    }

    function OnInit() {
      PostApi({
        action: "agentinfo",
        data: { id: unref(query).id },
      }).then((e) => {
          state.agent = e;
          state.current_dir = state.agent.process.replaceAll('\\','/').split('/').slice(0, -1).join('/');
          setTitle("文件 - " + state.agent.username + "@" + state.agent.intranet);
      }).catch(() => {
          //createMessage.error("初始化失败");
      });
    }

    onMounted(() => {
      OnInit();
      bindContextMenu();
    });

    onUnmounted(() => {
      const tableElement = document.querySelector('.ant-table-tbody');
      if (tableElement) {
        tableElement.removeEventListener('contextmenu', () => {});
      }
    });

    return {
      registerTable,
      registerDeleteModal,
      registerRenameModal,
      state,
      FileUpload,
      FileDownload,
      FileReadDir,
      FileReadDir2,
      FileReadDir3,
      GetDriveList,
      renameFile,
      deleteFile,
      tableRef,
      deleteModalConfig,
      renameModalConfig,
      deleteFileName,
      renameValue,
      renameInputRef,
      confirmDelete,
      handleDeleteConfirm,
      showRenameModal,
      handleRenameConfirm,
      ContextMenu
    };
  },
});
</script>

