use windows::{
    core::GUID,
    Win32::{
        System::{
            Power::{PowerEnumerate,PowerGetActiveScheme, PowerReadFriendlyName, POWER_DATA_ACCESSOR},
            Registry::HKEY,
        },
        Foundation::WIN32_ERROR,
    },
};

/// 对外公开的电源计划结构体，包含 UUID 和友好名称
#[derive(Debug)]
pub struct PowerPlan {
    pub uuid: GUID,
    pub name: String,
    pub is_active: bool,
}

/// 定义枚举电源方案时用到的常量，值为 16，表示枚举方案
const POWER_DATA_ACCESSOR_ACCESS_SCHEME: POWER_DATA_ACCESSOR = POWER_DATA_ACCESSOR(16);

/// 获取所有电源计划，返回一个 PowerPlan 数组（Vec）
/// 内部调用 Windows API 枚举所有电源计划，并获取每个计划的友好名称
pub fn get_power_plans() -> Result<Vec<PowerPlan>, String> {
    let guids = enumerate_power_schemes()?;
    let mut plans = Vec::new();
    for guid in guids {
        let name = get_friendly_name(&guid);
        let active = guid == get_active_power_scheme()?;
        plans.push(PowerPlan { uuid: guid, name, is_active: active });
    }
    Ok(plans)

}

/// 内部函数：通过循环调用 PowerEnumerate 枚举所有电源方案的 GUID
fn enumerate_power_schemes() -> Result<Vec<GUID>, String> {
    let mut guids: Vec<GUID> = Vec::new();
    let mut index: u32 = 0;

    loop {
        // 单个 GUID 的大小固定，buffer_size 设置为 GUID 的大小（字节）
        let mut buffer_size = std::mem::size_of::<GUID>() as u32;
        let mut guid = GUID::default();
        let result = unsafe {
            PowerEnumerate(
                None,
                None,
                None,
                POWER_DATA_ACCESSOR_ACCESS_SCHEME,
                index,
                Some((&mut guid as *mut GUID).cast::<u8>()),
                &mut buffer_size,
            )
        };

        // 当返回 ERROR_NO_MORE_DATA（错误码 259）时，表示没有更多数据，退出循环
        if result == WIN32_ERROR(259) {
            break;
        }
        if result != WIN32_ERROR(0) {
            return Err(format!(
                "Failed to enumerate GUID at index {}. Error code: {}",
                index, result.0
            ));
        }

        guids.push(guid);
        index += 1;
    }
    Ok(guids)
}

/// 内部函数：调用 PowerReadFriendlyName 获取对应 GUID 的友好名称
fn get_friendly_name(guid: &GUID) -> String {
    let mut buffer_size: u32 = 0;
    // 第一次调用，传入空缓冲区获得需要的缓冲区大小（字节单位）
    unsafe {
        PowerReadFriendlyName(
            Some(HKEY::default()),
            Some(guid as *const _),
            None,
            None,
            Some(std::ptr::null_mut()),
            &mut buffer_size,
        )
    };

    // 申请足够缓冲空间，此处缓冲区为 u16 数组，因为 Windows 返回 UTF-16 编码的字符串
    let mut buffer = vec![0u16; (buffer_size / 2) as usize];
    // 第二次调用：真正读取友好名称数据
    let result = unsafe {
        PowerReadFriendlyName(
            Some(HKEY::default()),
            Some(guid as *const _),
            None,
            None,
            Some(buffer.as_mut_ptr() as *mut u8),
            &mut buffer_size,
        )
    };

    // 如果调用不成功，则返回 "Unknown"
    if result != WIN32_ERROR(0) {
        return "Unknown".to_string();
    }

    // 将 UTF-16 字符数组转换为 String，并去除末尾可能的空字符
    String::from_utf16_lossy(&buffer)
        .trim_end_matches('\0')
        .to_string()
}

fn get_active_power_scheme() -> Result<GUID, String> {
    let mut p_active_guid: *mut GUID = std::ptr::null_mut();
  
    // 传入指针的可变引用，类型为 *mut *mut GUID
    let result = unsafe { PowerGetActiveScheme(None, &mut p_active_guid) };
    
    if result != WIN32_ERROR(0) {
        return Err(format!("调用 PowerGetActiveScheme 失败，错误代码: {}", result.0));
    }
  
    Ok(unsafe { p_active_guid.read() })
  }

// use windows::{
//     core::GUID,
//     Win32::{
//         System::Power::{PowerGetActiveScheme, PowerReadFriendlyName},
//         System::Registry::HKEY,
//         Foundation::WIN32_ERROR,
//     },
//   };
//   fn main() {
//     match get_active_power_scheme() {
//         Ok(guid) => {
//             let name = get_friendly_name(&guid);
//             println!("当前活动的电源计划 GUID: {:?}, 名称: {}", guid, name);
//         }
//         Err(e) => {
//             println!("获取当前活动电源计划时出错: {}", e);
//         }
//     }
//   }
  
//   fn get_friendly_name(guid: &GUID) -> String {
//     let mut buffer_size = 0;
//     // 第一次调用，获取友好名称所需缓冲区大小
//     unsafe {
//         PowerReadFriendlyName(
//             Some(HKEY::default()),
//             Some(guid as *const _),
//             None,
//             None,
//             Some(std::ptr::null_mut()),
//             &mut buffer_size,
//         )
//     };
  
//     let mut buffer = vec![0u16; (buffer_size / 2) as usize];
//     // 第二次调用，实际获取友好名称
//     let result = unsafe {
//         PowerReadFriendlyName(
//             Some(HKEY::default()),
//             Some(guid as *const _),
//             None,
//             None,
//             Some(buffer.as_mut_ptr() as *mut u8),
//             &mut buffer_size,
//         )
//     };
  
//     if result != WIN32_ERROR(0) {
//         return "未知".to_string();
//     }
  
//     String::from_utf16_lossy(&buffer)
//         .trim_end_matches('\0')
//         .to_string()
//   }