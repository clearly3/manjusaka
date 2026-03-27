<template>
  <div>
    <BasicModal
      v-bind="$attrs"
      :width="800"
      title="生成NPC1"
      okText="下载"
      @register="registerModal"
      @ok="handleSubmit"
    >
      <BasicForm @register="registerForm" />
      NPC下载地址
      <Alert :message="state.url" />
    </BasicModal>
  </div>
</template>
<script lang="ts">
import { defineComponent, reactive, watchEffect, watch } from "vue";
import { BasicModal, useModalInner } from "/@/components/Modal";
import { BasicForm, useForm } from "/@/components/Form/index";
import { Alert } from "ant-design-vue";
import { useGlobSetting } from "/@/hooks/setting";
const { downloadUrl = "" } = useGlobSetting();

export default defineComponent({
  name: "RoleModal",
  components: {
    BasicModal,
    BasicForm,
    Alert,
  },
  emits: ["success", "register"],
  setup() {
    const state = reactive({
      mod: [1, 2, 1, 1],
      url: "",
    });
    const [registerForm, { resetFields, setFieldsValue, getFieldsValue }] = useForm({
      labelWidth: 120,
      schemas: [
        {
          field: "name",
          component: "Input",
          label: "项目名称",
          componentProps: {
            disabled: true,  // 正确的写法
          },
        },
        {
          field: "callback1",
          component: "Input",
          label: "回连地址",
          componentProps: {
            disabled: true,  // 正确的写法
          },
        },
        {
          field: "route",
          component: "Input",
          label: "项目路由",
          componentProps: {
            disabled: true,  // 正确的写法
          },
        },
        {
          field: "arch",
          label: "arch",
          component: "RadioButtonGroup",
          defaultValue: "x86_64",
          componentProps: (form) => ({
            options: [
              { label: "x86_64", value: "1" },
              { label: "x86", value: "2" },
              { label: "aarch64", value: "3" },
              { label: "arm", value: "4" },
              { label: "mips64", value: "5" },
              { label: "mips", value: "6" },
              { label: "loongarch64", value: "7" },
            ],
            onChange: (e) => {
              state.mod[2] = e;
              changeMod();
            },
          }),
        },
        {
          field: "platform",
          label: "NPC类型",
          component: "RadioButtonGroup",
          defaultValue: "exe",
          componentProps: (form) => ({
            options: [
              { label: "windows", value: "1" },
              { label: "linux", value: "2" },
              { label: "macos", value: "3" },
              //{ label: "android", value: "4" },
              //{ label: "ios", value: "5" },
            ],
            onChange: (e) => {
              state.mod[3] = e;
              changeMod();
            },
          }),
        },
      ],
      showResetButton: false,
      showSubmitButton: false,
    });

    const [registerModal, { setModalProps, closeModal }] = useModalInner(async (data) => {
      setFieldsValue(data);
      if (data.callback1 && data.route) {
        let mod = parseInt(state.mod.join(''));
        state.url = data.callback1.trim().replace(/\/+$/, '') + "/" + data.route + "/" + getstr(mod) + ".png";
      }
    });

    function changeMod() {
      let data = getFieldsValue();
      if (data.callback1 && data.route) {
        let mod = parseInt(state.mod.join(''));
        state.url = data.callback1.trim().replace(/\/+$/, '') + "/" + data.route + "/" + getstr(mod) + ".png";
      }
    }
    function handleSubmit() {
      window.open(state.url);
    }

    function getstr(n) {
        if (n > 9999) {return n.toString();}
        while (true) {
            const s = randstr(6);
            let p = BigInt(1);
            for (let c of s) { p = p * BigInt(c.charCodeAt(0)) % BigInt(10000);}
            if (p === BigInt(n)) {return s;}
        }
    }
    function randstr(length) {
        const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
        let result = '';
        for (let i = 0; i < length; i++) {
            result += chars.charAt(Math.floor(Math.random() * chars.length));
        }
        return result;
    }

    return {
      state,
      registerModal,
      registerForm,
      handleSubmit,
    };
  },
});
</script>
