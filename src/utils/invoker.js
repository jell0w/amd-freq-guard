import { invoke } from "@tauri-apps/api/core";
// import { useToast } from "primevue/usetoast";
import toast from "./toast";

// 创建一个 toast 实例
// let toast = null;

/**
 * invoke 包装器，用于统一处理错误

 * @param {string} command - 要调用的命令名称
 * @param {object} [args] - 命令参数
 * @returns {Promise} - 返回命令执行结果
 */
export async function invoker(command, args = undefined, ignoreErrorToast = false) {
  // if (!toast) {
  //     toast = useToast();
  // }
  try {
    return await invoke(command, args);
  } catch (error) {
    if (!ignoreErrorToast) {
      // 显示错误提示
      toast.add({
        severity: 'error',
        summary: '操作失败',
        detail: error.toString(),
        life: 5000
      });
    }
    // 继续抛出错误，让调用者可以进行额外处理
    throw error;
  }
} 