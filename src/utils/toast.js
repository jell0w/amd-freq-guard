// toast.js
import app from "../app"
const getToast = () => app.config.globalProperties.$toast
const toast = {
	add(param) {
		getToast().add(param)
	}
}
export default toast