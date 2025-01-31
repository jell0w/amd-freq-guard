import {
  isPermissionGranted,
  requestPermission,
  sendNotification
} from '@tauri-apps/plugin-notification'

async function checkPermission() {
  if (!(await isPermissionGranted())) {
    return (await requestPermission()) === 'granted'
  }
  return true
}

async function enqueueNotification(title, body) {
  if (!(await checkPermission())) {
    return
  }
  if(typeof title !== 'string') {
    title = title.toString()
  }
  if(typeof body !== 'string') {
    body = body.toString()
  }
  sendNotification({ title, body })
}

export const showNotification = async (title, body) => {
  await enqueueNotification(title, body);
};

export default {
  showNotification
} 