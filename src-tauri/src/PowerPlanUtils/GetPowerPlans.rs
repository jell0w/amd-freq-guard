use windows::{
    core::{Error, GUID},
    Win32::{
        Foundation::WIN32_ERROR,
        System::{
            Power::{
                PowerEnumerate, PowerGetActiveScheme, PowerReadACValue, PowerReadDCValue,
                PowerReadFriendlyName, PowerReadPossibleFriendlyName, PowerReadPossibleValue,
                PowerReadSettingAttributes, PowerReadValueMax, PowerReadValueMin,
                PowerReadValueUnitsSpecifier, PowerSetActiveScheme, PowerWriteACValueIndex,
                PowerWriteDCValueIndex, PowerWriteSettingAttributes, POWER_DATA_ACCESSOR,
            },
            Registry::HKEY,
        },

    },
};

// use serde;
use serde::Serialize;
use serde_json;

use log::info;

/// 对外公开的电源计划结构体，包含 UUID 和友好名称
#[derive(Debug, Serialize)]
pub struct PowerPlan {
    #[serde(serialize_with = "serialize_guid")]
    pub uuid: GUID,
    pub name: String,
    pub is_active: bool,
}

fn serialize_guid<S>(guid: &GUID, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("{:?}", guid))
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerSubgroup {
    #[serde(serialize_with = "serialize_guid")]
    pub uuid: GUID,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PowerSetting {
    #[serde(serialize_with = "serialize_guid")]
    pub uuid: GUID,
    pub name: String,
}

/// 定义枚举电源方案时用到的常量，值为 16，表示枚举方案
const POWER_DATA_ACCESSOR_ACCESS_SCHEME: POWER_DATA_ACCESSOR = POWER_DATA_ACCESSOR(16);

const POWER_DATA_ACCESSOR_ACCESS_SUBGROUP: POWER_DATA_ACCESSOR = POWER_DATA_ACCESSOR(17);
const POWER_DATA_ACCESSOR_ACCESS_SETTING: POWER_DATA_ACCESSOR = POWER_DATA_ACCESSOR(18);

/// 获取所有电源计划，返回一个 PowerPlan 数组（Vec）
/// 内部调用 Windows API 枚举所有电源计划，并获取每个计划的友好名称
pub fn get_power_plans() -> Result<Vec<PowerPlan>, String> {
    let guids = enumerate_power_schemes()?;
    let mut plans = Vec::new();
    for guid in guids {
        let name = get_friendly_name(Some(&guid), None, None);
        let active = guid == get_active_power_scheme()?;
        plans.push(PowerPlan {
            uuid: guid,
            name,
            is_active: active,
        });
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

//传入电源计划的GUID字符串，返回该电源计划下的所有子组
pub fn enumerate_power_scheme_subgroups(guid_str: &str) -> Result<Vec<PowerSubgroup>, String> {
    // let mut guids: Vec<GUID> = Vec::new();
    let mut subgroups: Vec<PowerSubgroup> = Vec::new();
    let mut index: u32 = 0;

    // 移除短横线
    let guid_str = guid_str.replace("-", "");

    // 将字符串转换为 u128
    let guid_u128 = u128::from_str_radix(&guid_str, 16).expect("Invalid GUID string");

    // 将 u128 转换为 GUID
    let scheme_guid = GUID::from_u128(guid_u128);

    println!("scheme_guid: {:?}", scheme_guid);

    loop {
        // 单个 GUID 的大小固定，buffer_size 设置为 GUID 的大小（字节）
        let mut buffer_size = std::mem::size_of::<GUID>() as u32;
        let mut guid = GUID::default();
        let result = unsafe {
            PowerEnumerate(
                None,
                Some(&scheme_guid),
                None,
                POWER_DATA_ACCESSOR_ACCESS_SUBGROUP,
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

        // guids.push(guid);
        subgroups.push(PowerSubgroup {
            uuid: guid,
            name: get_friendly_name(Some(&scheme_guid), Some(&guid), None),
        });
        index += 1;
    }

    Ok(subgroups)
}

pub fn enumerate_power_scheme_settings(
    guid_str: &str,
    subgroup_guid_str: &str,
) -> Result<Vec<PowerSetting>, String> {
    let mut settings: Vec<PowerSetting> = Vec::new();
    let mut index: u32 = 0;

    //将两个guid字符串转换为GUID
    //先移除短横线
    let guid_str = guid_str.replace("-", "");
    let subgroup_guid_str = subgroup_guid_str.replace("-", "");

    //将字符串转换为u128
    let guid_u128 = u128::from_str_radix(&guid_str, 16).expect("Invalid GUID string");
    let subgroup_guid_u128 =
        u128::from_str_radix(&subgroup_guid_str, 16).expect("Invalid GUID string");

    //将u128转换为GUID
    let guid = GUID::from_u128(guid_u128);
    let subgroup_guid = GUID::from_u128(subgroup_guid_u128);

    //循环调用PowerEnumerate
    loop {
        let mut buffer_size = std::mem::size_of::<GUID>() as u32;
        let mut guid = GUID::default();
        let result = unsafe {
            PowerEnumerate(
                None,
                Some(&guid),
                Some(&subgroup_guid),
                POWER_DATA_ACCESSOR_ACCESS_SETTING,
                index,
                Some((&mut guid as *mut GUID).cast::<u8>()),
                &mut buffer_size,
            )
        };

        if result == WIN32_ERROR(259) {
            break;
        }
        if result != WIN32_ERROR(0) {
            return Err(format!(
                "Failed to enumerate GUID at index {}. Error code: {}",
                index, result.0
            ));
            // println!("Failed to enumerate GUID at index {}. Error code: {}", index, result.0);
        }

        index += 1;
        // println!("guid: {:?}", guid);
        // println!("get_friendly_name: {:?}", get_friendly_name(Some(&guid),Some(&subgroup_guid),Some(&guid)));
        settings.push(PowerSetting {
            uuid: guid,
            name: get_friendly_name(Some(&guid), Some(&subgroup_guid), Some(&guid)),
        });
    }
    Ok(settings)
}

pub fn get_power_setting_ac_value(
    guid: GUID,
    subgroup_guid: GUID,
    setting_guid: GUID,
) -> Result<u32, String> {
    let mut value_type: u32 = 0;
    let mut buffer_size: u32 = 0;

    // 第一次调用，未传入缓冲区，仅获取所需大小
    let result = unsafe {
        PowerReadACValue(
            None,
            Some(&guid),
            Some(&subgroup_guid),
            Some(&setting_guid),
            Some(&mut value_type),
            None,
            Some(&mut buffer_size),
        )
    };

    // 如果结果不是 ERROR_MORE_DATA 或 0，则出错
    if result != WIN32_ERROR(234) && result != WIN32_ERROR(0) {
        // println!("读取值失败，错误码: {}", result.0);
        return Err(format!("读取值失败，错误码: {}", result.0));
    }

    // 分配足够大小的缓冲区
    let mut buf = vec![0u8; buffer_size as usize];

    // 第二次调用，传入缓冲区读取数据
    let result = unsafe {
        PowerReadACValue(
            None,
            Some(&guid),
            Some(&subgroup_guid),
            Some(&setting_guid),
            Some(&mut value_type),
            Some(buf.as_mut_ptr()),
            Some(&mut buffer_size),
        )
    };

    if result != WIN32_ERROR(0) {
        // println!("读取值失败，错误码: {}", result.0);
        return Err(format!("读取值失败，错误码: {}", result.0));
    }

    if buf.len() >= 4 {
        let num = u32::from_le_bytes(buf[..4].try_into().unwrap());
        // println!("转换后的DWORD值：{}", num);
        return Ok(num);
    } else {
        // println!("缓冲区长度不足，无法转换为u32");
        return Err("缓冲区长度不足，无法转换为u32".to_string());
    }
}

pub fn get_power_setting_dc_value(
    guid: GUID,
    subgroup_guid: GUID,
    setting_guid: GUID,
) -> Result<u32, String> {
    let mut value_type: u32 = 0;
    let mut buffer_size: u32 = 0;

    let result = unsafe {
        PowerReadDCValue(
            None,
            Some(&guid),
            Some(&subgroup_guid),
            Some(&setting_guid),
            Some(&mut value_type),
            None,
            &mut buffer_size,
        )
    };

    if result != WIN32_ERROR(0) {
        return Err(format!("读取值失败，错误码: {}", result.0));
    }

    let mut buf = vec![0u8; buffer_size as usize];

    let result = unsafe {
        PowerReadDCValue(
            None,
            Some(&guid),
            Some(&subgroup_guid),
            Some(&setting_guid),
            Some(&mut value_type),
            Some(buf.as_mut_ptr()),
            &mut buffer_size,
        )
    };

    if result != WIN32_ERROR(0) {
        return Err(format!("读取值失败，错误码: {}", result.0));
    }

    if buf.len() >= 4 {
        let num = u32::from_le_bytes(buf[..4].try_into().unwrap());
        return Ok(num);
    } else {
        return Err("缓冲区长度不足，无法转换为u32".to_string());
    }
}

pub fn set_power_setting_ac_value(
    guid: GUID,
    subgroup_guid: GUID,
    setting_guid: GUID,
    ac_value_index: u32,
) -> Result<(), String> {
    // 调用 PowerWriteACValueIndex 设置新的 AC 电源设置
    let result = unsafe {
        PowerWriteACValueIndex(
            None,                 // 根电源键（可以设置为 None）
            &guid,                // 电源计划的 GUID
            Some(&subgroup_guid), // 电源子组的 GUID
            Some(&setting_guid),  // 电源设置的 GUID
            ac_value_index,       // 设置新的 AC 值索引
        )
    };

    // 判断函数调用结果
    if result != WIN32_ERROR(0) {
        return Err(format!("设置AC电源值失败，错误码: {}", result.0));
    }
    Ok(())
}

pub fn set_power_setting_dc_value(
    guid: GUID,
    subgroup_guid: GUID,
    setting_guid: GUID,
    dc_value_index: u32,
) -> Result<(), String> {
    // 调用 PowerWriteDCValueIndex 设置新的 DC 电源设置
    let result = unsafe {
        PowerWriteDCValueIndex(
            None,                 // 根电源键（可以设置为 None）
            &guid,                // 电源计划的 GUID
            Some(&subgroup_guid), // 电源子组的 GUID
            Some(&setting_guid),  // 电源设置的 GUID
            dc_value_index,       // 设置新的 DC 值索引
        )
    };

    // 判断函数调用结果
    if result != 0 {
        return Err(format!("设置DC电源值失败，错误码: {}", result));
    }

    Ok(())
}

pub fn write_value_set(
    guid: &str,
    subgroup_guid: &str,
    setting_guid: &str,
    ac_value: u32,
    dc_value: u32,
) -> Result<(), String> {
    let guid_str = guid.replace("-", "");
    let subgroup_guid_str = subgroup_guid.replace("-", "");
    let setting_guid_str = setting_guid.replace("-", "");

    let guid_u128 = u128::from_str_radix(&guid_str, 16).expect("Invalid GUID string");
    let subgroup_guid_u128 =
        u128::from_str_radix(&subgroup_guid_str, 16).expect("Invalid GUID string");
    let setting_guid_u128 =
        u128::from_str_radix(&setting_guid_str, 16).expect("Invalid GUID string");

    let guid = GUID::from_u128(guid_u128);
    let subgroup_guid = GUID::from_u128(subgroup_guid_u128);
    let setting_guid = GUID::from_u128(setting_guid_u128);

    set_power_setting_ac_value(guid, subgroup_guid, setting_guid, ac_value)?;
    set_power_setting_dc_value(guid, subgroup_guid, setting_guid, dc_value)?;
    //如果当前修改的电源计划是活动电源计划，则更新活动电源计划
    if Ok::<_, String>(get_active_power_scheme()?) == Ok(guid) {
        info!("当前修改的电源计划是活动电源计划，更新活动电源计划");
        activate_power_scheme(guid)?;
    }
    Ok(())
}


#[derive(Debug, Clone, Serialize)]
pub struct PowerSettingValue {
    pub ac_value: u32,
    pub dc_value: u32,
}

pub fn get_power_setting_value(
    guid_str: &str,
    subgroup_guid_str: &str,
    setting_guid_str: &str,
) -> Result<PowerSettingValue, String> {
    let guid_str = guid_str.replace("-", "");
    let subgroup_guid_str = subgroup_guid_str.replace("-", "");
    let setting_guid_str = setting_guid_str.replace("-", "");

    let guid_u128 = u128::from_str_radix(&guid_str, 16).expect("Invalid GUID string");
    let subgroup_guid_u128 =
        u128::from_str_radix(&subgroup_guid_str, 16).expect("Invalid GUID string");
    let setting_guid_u128 =
        u128::from_str_radix(&setting_guid_str, 16).expect("Invalid GUID string");

    let guid = GUID::from_u128(guid_u128);
    let subgroup_guid = GUID::from_u128(subgroup_guid_u128);
    let setting_guid = GUID::from_u128(setting_guid_u128);

    let ac_value = get_power_setting_ac_value(guid, subgroup_guid, setting_guid)?;
    let dc_value = get_power_setting_dc_value(guid, subgroup_guid, setting_guid)?;
    Ok(PowerSettingValue { ac_value, dc_value })
}

pub fn enumerate_possible_values(
    subgroup_guid_str: &str,
    setting_guid_str: &str,
) -> Result<Vec<u32>, String> {
    // 移除子组和设置 GUID 字符串中的短横线
    let subgroup_guid_clean = subgroup_guid_str.replace("-", "");
    let setting_guid_clean = setting_guid_str.replace("-", "");

    // 将字符串转换为 u128，再构造 GUID
    let subgroup_guid_u128 = u128::from_str_radix(&subgroup_guid_clean, 16)
        .map_err(|e| format!("无效的子组 GUID 字符串: {}", e))?;
    let setting_guid_u128 = u128::from_str_radix(&setting_guid_clean, 16)
        .map_err(|e| format!("无效的设置 GUID 字符串: {}", e))?;
    let subgroup_guid = windows::core::GUID::from_u128(subgroup_guid_u128);
    let setting_guid = windows::core::GUID::from_u128(setting_guid_u128);

    let mut possible_values = Vec::new();
    let mut index: u32 = 0;

    loop {
        // 初始缓冲区长度设置为 4 字节（DWORD 大小）
        let mut buffer_size: u32 = std::mem::size_of::<u32>() as u32;
        let mut buffer = vec![0u8; buffer_size as usize];
        let mut value_type: u32 = 0;

        // 调用 PowerReadPossibleValue 获取对应索引的可能取值
        let result = unsafe {
            PowerReadPossibleValue(
                None,
                Some(&subgroup_guid),
                Some(&setting_guid),
                Some(&mut value_type),
                index,
                Some(buffer.as_mut_ptr()),
                &mut buffer_size,
            )
        };

        // 当返回 ERROR_NO_MORE_DATA (259)或 234 时，表示没有更多数据，退出循环
        if result == windows::Win32::Foundation::WIN32_ERROR(259)
            || result == windows::Win32::Foundation::WIN32_ERROR(2)
        {
            break;
        }
        if result != windows::Win32::Foundation::WIN32_ERROR(0) {
            println!(
                "enumerate_possible_values读取可能取值失败，索引 {} 错误码: {}",
                index, result.0
            );
            return Err(format!(
                "读取可能取值失败，索引 {} 错误码: {}",
                index, result.0
            ));
        }

        if buffer_size < 4 {
            return Err("缓冲区不足，无法转换为 u32".to_string());
        }
        let num = u32::from_le_bytes(buffer[..4].try_into().unwrap());
        println!("num: {:?}", num);
        possible_values.push(num);
        index += 1;
    }
    Ok(possible_values)
}

/// 内部函数：调用 PowerReadFriendlyName 获取对应 GUID 的友好名称
//subgroupGuid 为空时，为none
fn get_friendly_name(
    powerPlanGuid: Option<&GUID>,
    subgroupGuid: Option<&GUID>,
    settingGuid: Option<&GUID>,
) -> String {
    let mut buffer_size: u32 = 0;
    // 第一次调用，传入空缓冲区获得需要的缓冲区大小（字节单位）
    unsafe {
        PowerReadFriendlyName(
            Some(HKEY::default()),
            powerPlanGuid.map(|guid| guid as *const _),
            subgroupGuid.map(|guid| guid as *const _),
            settingGuid.map(|guid| guid as *const _),
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
            powerPlanGuid.map(|guid| guid as *const _),
            subgroupGuid.map(|guid| guid as *const _),
            settingGuid.map(|guid| guid as *const _),
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
        return Err(format!(
            "调用 PowerGetActiveScheme 失败，错误代码: {}",
            result.0
        ));
    }

    Ok(unsafe { p_active_guid.read() })
}

fn activate_power_scheme(guid: GUID) -> Result<(), String> {
    let result = unsafe {
        PowerSetActiveScheme(
            None,  // 根电源键（可以设置为 None）
            Some(&guid), // 电源计划的 GUID
        )
    };


    if result != WIN32_ERROR(0) {
        return Err(format!("激活电源计划失败，错误码: {}", result.0));
    }


    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub enum SettingType {
    Enumerated,
    Range,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingData {
    pub value: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PossibleSetting {
    pub setting_type: SettingType,
    pub data: Vec<SettingData>,
    pub unit: Option<String>,
}

pub fn get_power_setting_range(
    subgroup_guid_str: &str,
    setting_guid_str: &str,
) -> Result<Option<(u32, u32)>, String> {
    // 移除横线并转换 GUID
    let subgroup_guid_clean = subgroup_guid_str.replace("-", "");
    let setting_guid_clean = setting_guid_str.replace("-", "");

    let subgroup_guid_u128 = u128::from_str_radix(&subgroup_guid_clean, 16)
        .map_err(|e| format!("无效的子组 GUID 字符串: {}", e))?;
    let setting_guid_u128 = u128::from_str_radix(&setting_guid_clean, 16)
        .map_err(|e| format!("无效的设置 GUID 字符串: {}", e))?;

    let subgroup_guid = windows::core::GUID::from_u128(subgroup_guid_u128);
    let setting_guid = windows::core::GUID::from_u128(setting_guid_u128);

    let mut min_value: u32 = 0;
    let mut max_value: u32 = 0;

    // 尝试读取最小值和最大值
    let min_result = unsafe {
        PowerReadValueMin(
            None,
            Some(&subgroup_guid),
            Some(&setting_guid),
            &mut min_value,
        )
    };

    let max_result = unsafe {
        PowerReadValueMax(
            None,
            Some(&subgroup_guid),
            Some(&setting_guid),
            &mut max_value,
        )
    };

    // 如果两个值都读取成功，返回范围
    if min_result == WIN32_ERROR(0) && max_result == WIN32_ERROR(0) {
        Ok(Some((min_value, max_value)))
    } else {
        Ok(None) // 返回 None 表示这可能是一个枚举值而不是范围值
    }
}

// 新增读取单位的辅助函数
fn read_units_specifier(subgroup_guid: &GUID, setting_guid: &GUID) -> Option<String> {
    let mut buffer_size: u32 = 0;

    // 第一次调用获取所需缓冲区大小
    unsafe {
        PowerReadValueUnitsSpecifier(
            None,
            Some(subgroup_guid),
            Some(setting_guid),
            None,
            &mut buffer_size,
        )
    };

    // 如果缓冲区大小为 0，说明没有单位
    if buffer_size == 0 {
        return None;
    }

    // 分配缓冲区（UTF-16 字符串）
    let mut buffer: Vec<u16> = vec![0; (buffer_size / 2) as usize];
    let result = unsafe {
        PowerReadValueUnitsSpecifier(
            None,
            Some(subgroup_guid),
            Some(setting_guid),
            Some(buffer.as_mut_ptr() as *mut u8),
            &mut buffer_size,
        )
    };

    // println!("result: {:?}", result);

    if result != WIN32_ERROR(0) {
        return None;
    }

    // 转换为 String 并去除结尾的空字符
    Some(
        String::from_utf16_lossy(&buffer)
            .trim_end_matches('\0')
            .to_string(),
    )
}

pub fn enumerate_possible_settings(
    subgroup_guid_str: &str,
    setting_guid_str: &str,
) -> Result<PossibleSetting, String> {
    // 移除子组和设置 GUID 字符串中的横线
    let subgroup_guid_clean = subgroup_guid_str.replace("-", "");
    let setting_guid_clean = setting_guid_str.replace("-", "");

    // 将 GUID 字符串转换为 u128，再构造 GUID
    let subgroup_guid_u128 = u128::from_str_radix(&subgroup_guid_clean, 16)
        .map_err(|e| format!("无效的子组 GUID 字符串: {}", e))?;
    let setting_guid_u128 = u128::from_str_radix(&setting_guid_clean, 16)
        .map_err(|e| format!("无效的设置 GUID 字符串: {}", e))?;
    let subgroup_guid = windows::core::GUID::from_u128(subgroup_guid_u128);
    let setting_guid = windows::core::GUID::from_u128(setting_guid_u128);

    // 读取单位说明
    let unit = read_units_specifier(&subgroup_guid, &setting_guid);

    // 先尝试获取范围值
    if let Ok(Some((min, max))) = get_power_setting_range(subgroup_guid_str, setting_guid_str) {
        return Ok(PossibleSetting {
            setting_type: SettingType::Range,
            data: vec![
                SettingData {
                    value: min,
                    name: "min".to_string(),
                },
                SettingData {
                    value: max,
                    name: "max".to_string(),
                },
            ],
            unit,
        });
    }

    let mut setting_data = Vec::new();
    let mut index: u32 = 0;

    loop {
        // 读取可能取值
        let mut value_buffer_size: u32 = std::mem::size_of::<u32>() as u32;
        let mut value_buf = vec![0u8; value_buffer_size as usize];
        let mut value_type: u32 = 0;

        loop {
            let result_value = unsafe {
                PowerReadPossibleValue(
                    None,
                    Some(&subgroup_guid),
                    Some(&setting_guid),
                    Some(&mut value_type),
                    index,
                    Some(value_buf.as_mut_ptr()),
                    &mut value_buffer_size,
                )
            };

            if result_value == WIN32_ERROR(234) {
                // ERROR_MORE_DATA
                // 扩大缓冲区并重试
                value_buf = vec![0u8; value_buffer_size as usize];
                continue;
            }

            // 当返回 ERROR_NO_MORE_DATA (259) 时表示没有更多数据
            if result_value == windows::Win32::Foundation::WIN32_ERROR(259)
                || result_value == windows::Win32::Foundation::WIN32_ERROR(2)
            {
                return Ok(PossibleSetting {
                    setting_type: SettingType::Enumerated,
                    data: setting_data,
                    unit,
                });
            }
            if result_value != windows::Win32::Foundation::WIN32_ERROR(0) {
                return Err(format!(
                    "读取可能取值失败，索引 {} 错误码: {}",
                    index, result_value.0
                ));
            }
            break; // 成功读取数据，退出内层循环
        }

        let value = u32::from_le_bytes(value_buf[..4].try_into().unwrap());

        // 读取友好名称
        let mut name_buffer_size: u32 = 0;

        // 第一次调用获取所需缓冲区大小
        unsafe {
            PowerReadPossibleFriendlyName(
                None,
                Some(&subgroup_guid),
                Some(&setting_guid),
                index,
                None,
                &mut name_buffer_size,
            );
        }

        let mut name_buffer = vec![0u8; name_buffer_size as usize];

        // 循环处理 ERROR_MORE_DATA
        loop {
            let result_name = unsafe {
                PowerReadPossibleFriendlyName(
                    None,
                    Some(&subgroup_guid),
                    Some(&setting_guid),
                    index,
                    Some(name_buffer.as_mut_ptr()),
                    &mut name_buffer_size,
                )
            };

            if result_name == WIN32_ERROR(234) {
                // ERROR_MORE_DATA
                // 扩大缓冲区并重试
                name_buffer = vec![0u8; name_buffer_size as usize];
                continue;
            }

            if result_name != windows::Win32::Foundation::WIN32_ERROR(0) {
                setting_data.push(SettingData {
                    value,
                    name: "Unknown".to_string(),
                });
                break;
            }

            // 成功读取数据
            let name = String::from_utf16_lossy(
                &name_buffer[..name_buffer_size as usize]
                    .chunks(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect::<Vec<u16>>(),
            )
            .trim_end_matches('\0')
            .to_string();

            setting_data.push(SettingData { value, name });
            break;
        }

        index += 1;
    }
}

pub fn read_setting_attributes(
    subgroup_guid_str: &str,
    setting_guid_str: &str,
) -> Result<u32, String> {
    // 移除横线并转换 GUID
    let subgroup_guid_clean = subgroup_guid_str.replace("-", "");
    let setting_guid_clean = setting_guid_str.replace("-", "");

    let subgroup_guid_u128 = u128::from_str_radix(&subgroup_guid_clean, 16)
        .map_err(|e| format!("无效的子组 GUID 字符串: {}", e))?;
    let setting_guid_u128 = u128::from_str_radix(&setting_guid_clean, 16)
        .map_err(|e| format!("无效的设置 GUID 字符串: {}", e))?;

    let subgroup_guid = windows::core::GUID::from_u128(subgroup_guid_u128);
    let setting_guid = windows::core::GUID::from_u128(setting_guid_u128);

    // 调用 API 获取属性
    let attributes =
        unsafe { PowerReadSettingAttributes(Some(&subgroup_guid), Some(&setting_guid)) };

    Ok(attributes)
}

pub fn write_setting_attributes(
    subgroup_guid_str: &str,
    setting_guid_str: &str,
    attributes: u32,
) -> Result<(), String> {
    // 移除横线并转换 GUID
    let subgroup_guid_clean = subgroup_guid_str.replace("-", "");
    let setting_guid_clean = setting_guid_str.replace("-", "");

    let subgroup_guid_u128 = u128::from_str_radix(&subgroup_guid_clean, 16)
        .map_err(|e| format!("无效的子组 GUID 字符串: {}", e))?;
    let setting_guid_u128 = u128::from_str_radix(&setting_guid_clean, 16)
        .map_err(|e| format!("无效的设置 GUID 字符串: {}", e))?;

    let subgroup_guid = windows::core::GUID::from_u128(subgroup_guid_u128);
    let setting_guid = windows::core::GUID::from_u128(setting_guid_u128);

    // 调用 API 设置属性
    let result = unsafe {
        PowerWriteSettingAttributes(Some(&subgroup_guid), Some(&setting_guid), attributes)
    };

    if result != WIN32_ERROR(0) {
        return Err(format!("设置属性失败，错误码: {}", result.0));
    }

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct UnifiedPowerSetting {
    pub setting: PowerSetting,
    pub current_value: PowerSettingValue,
    pub possible_values: PossibleSetting,
    pub attributes: u32,
}

#[derive(Debug, Serialize)]
pub struct UnifiedPowerSubgroup {
    pub subgroup: PowerSubgroup,
    pub settings: Vec<UnifiedPowerSetting>,
}

#[derive(Debug, Serialize)]
pub struct UnifiedPowerScheme {
    pub scheme: PowerPlan,
    pub subgroups: Vec<UnifiedPowerSubgroup>,
}

pub fn get_unified_power_scheme(guid_str_input: &str) -> Result<UnifiedPowerScheme, String> {
    // 先把传入的guid_str_input转换为全大写
    let guid_str = guid_str_input.to_uppercase();

    // 获取电源计划基本信息
    let plans = get_power_plans()?;
    let scheme = plans
        .into_iter()
        .find(|p| format!("{:?}", p.uuid).contains(&guid_str))
        .ok_or_else(|| "未找到指定的电源计划".to_string())?;

    // 获取所有子组
    let subgroups = enumerate_power_scheme_subgroups(&guid_str)?;

    // 为每个子组获取设置
    let mut unified_subgroups = Vec::new();
    for subgroup in subgroups {
        let subgroup_guid_str = format!("{:?}", subgroup.uuid);
        let settings = enumerate_power_scheme_settings(&guid_str, &subgroup_guid_str)?;

        let mut unified_settings = Vec::new();
        for setting in settings {
            let setting_guid_str = format!("{:?}", setting.uuid);

            // 获取当前值
            let current_value =
                get_power_setting_value(&guid_str, &subgroup_guid_str, &setting_guid_str)?;

            // 获取可能的值
            let possible_values =
                enumerate_possible_settings(&subgroup_guid_str, &setting_guid_str)?;

            // 读取属性
            let attributes = read_setting_attributes(&subgroup_guid_str, &setting_guid_str)?;

            unified_settings.push(UnifiedPowerSetting {
                setting,
                current_value,
                possible_values,
                attributes,
            });
        }

        unified_subgroups.push(UnifiedPowerSubgroup {
            subgroup,
            settings: unified_settings,
        });
    }

    Ok(UnifiedPowerScheme {
        scheme,
        subgroups: unified_subgroups,
    })
}

pub fn get_unified_power_scheme_json_by_scheme_guid(
    guid_str_input: &str,
) -> Result<String, String> {
    let scheme = get_unified_power_scheme(guid_str_input)?;
    let json = serde_json::to_string_pretty(&scheme).map_err(|e| e.to_string())?;
    Ok(json)
}

#[cfg(test)]
mod tests {

    use super::*;
    use serde_json::to_string_pretty; // 使用 pretty 格式化输出

    // #[test]
    fn testSomething() {
        let test_guid_str = "8bd00add-abf1-47cf-98b4-0e38e7999415";

        let result_unified_power_scheme = get_unified_power_scheme(test_guid_str);
        match result_unified_power_scheme {
            Ok(scheme) => match to_string_pretty(&scheme) {
                Ok(json) => println!("JSON输出:\n{}", json),
                Err(e) => println!("JSON序列化失败: {}", e),
            },
            Err(e) => println!("获取统一电源方案失败: {}", e),
        }
        return;

        let test_subgroup_guid_str = "54533251-82be-4824-96c1-47b60b740d00";
        let test_setting_guid_str = "94D3A615-A899-4AC5-AE2B-E4D8F634367F";

        // 读取当前属性

        let result_attributes =
            read_setting_attributes(test_subgroup_guid_str, test_setting_guid_str);
        println!("当前属性值: {:?}", result_attributes);

        if let Ok(current_attributes) = result_attributes {
            // 如果当前是隐藏的（值为1），则设置为显示（值为0）
            // 如果当前是显示的（值为0），则设置为隐藏（值为1）
            let new_attributes = if current_attributes == 1 { 0 } else { 1 };

            let write_result = write_setting_attributes(
                test_subgroup_guid_str,
                test_setting_guid_str,
                new_attributes,
            );
            println!("设置新属性值 {} 的结果: {:?}", new_attributes, write_result);
        }

        let result_enumerate_power_scheme_settings =
            enumerate_power_scheme_settings(test_guid_str, test_subgroup_guid_str);
        println!(
            "result_enumerate_power_scheme_settings: {:?}",
            result_enumerate_power_scheme_settings
        );

        let result_setting_value =
            get_power_setting_value(test_guid_str, test_subgroup_guid_str, test_setting_guid_str);
        println!("result_setting_value: {:?}", result_setting_value);

        let result_possible_settings =
            enumerate_possible_settings(test_subgroup_guid_str, test_setting_guid_str);
        println!("result_possible_settings: {:?}", result_possible_settings);
    }

    #[test]
    fn test_set_power_setting_ac_value() {
        let test_guid_str = "eeb9e234-a391-4fdd-aa49-50f262128015";
        let test_subgroup_guid_str = "0012ee47-9041-4b5d-9b77-535fba8b1442";
        let test_setting_guid_str = "6738e2c4-e8a5-4a42-b16a-e040e769756e";

        let test_ac_value = 0;
        let test_dc_value = 0;

        let result_write_value_set = write_value_set(
            test_guid_str,
            test_subgroup_guid_str,
            test_setting_guid_str,
            test_ac_value,
            test_dc_value,
        );
        println!("result_write_value_set: {:?}", result_write_value_set);
    }
}
