import { invoker } from './invoker';

let constants = null;

export async function initConstants() {
    if (constants === null) {
        constants = await invoker('get_constants');
        console.log({ constants })
    }
    return constants;
}

export async function getConstant(key) {
    await initConstants();
    return constants[key];
}

export async function getGithubRepoURL() {
    return await getConstant('GITHUB_REPO_URL');
}

// 为了方便使用，也提供直接访问的方式
// export const GITHUB_REPO_URL = await getConstant('GITHUB_REPO');