use encoding_rs::GBK;
use serde::{Deserialize, Serialize};
use std::process::Command;
use uuid::Uuid;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerPlan {
    pub guid: String,
    pub name: String,
    pub is_active: bool,
}

pub fn get_power_plans() -> Result<Vec<PowerPlan>, String> {
    let output = Command::new("powercfg")
        .args(["/list"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    // 使用 GBK 解码
    let (cow, _encoding_used, had_errors) = GBK.decode(&output.stdout);
    if had_errors {
        return Err("GBK 解码失败".to_string());
    }
    let output_str = cow.into_owned();

    // println!("原始输出:\n{}", output_str);

    let plans = parse_power_plans(&output_str)?;

    // println!("解析结果:");
    // for plan in &plans {
    //     println!("GUID: {}", plan.guid);
    //     println!("名称: {}", plan.name);
    //     println!("是否活动: {}", plan.is_active);
    //     println!("---");
    // }

    Ok(plans)
}

pub fn set_active_plan(guid: &str) -> Result<(), String> {
    let output = Command::new("powercfg")
        .args(["/setactive", guid])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("设置活动计划失败: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("设置活动计划失败: {}", error));
    }

    Ok(())
}

fn parse_power_plans(output: &str) -> Result<Vec<PowerPlan>, String> {
    let mut plans = Vec::new();
    let mut found_header = false;

    // println!("开始解析行:");
    for line in output.lines() {
        // println!("处理行: {}", line);

        // 跳过空行
        if line.trim().is_empty() {
            // println!("跳过空行");
            continue;
        }

        // 检查是否找到头部分隔线
        if line.contains("-----------------------------------") {
            // println!("找到分隔线");
            found_header = true;
            continue;
        }

        // 解析计划信息
        if found_header && line.contains("GUID:") {
            // println!("解析GUID行");
            if let Some(plan) = parse_plan_line(line) {
                // println!("成功解析计划: {} ({})", plan.name, plan.guid);
                plans.push(plan);
            }
        }
    }

    Ok(plans)
}

fn parse_plan_line(line: &str) -> Option<PowerPlan> {
    let line = line.trim();

    // 查找 GUID 部分
    let guid_start = line.find("GUID:")? + 5;
    let remaining = &line[guid_start..];

    // 使用多个空格作为分隔符来分割 GUID 和名称部分
    let parts: Vec<&str> = remaining.split("  ").filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }

    let guid = parts[0].trim().to_string();

    // 获取名称部分（可能包含星号）
    let name_part = parts.get(1).map(|s| *s).unwrap_or("").trim();
    let is_active = name_part.ends_with('*');

    // 处理名称，移除括号和星号
    let name = if let Some(start) = name_part.find('(') {
        if let Some(end) = name_part.rfind(')') {
            // 提取括号中的内容
            let inner = &name_part[start + 1..end];
            // 移除可能的星号
            inner.trim_end_matches('*').trim().to_string()
        } else {
            name_part.trim_end_matches('*').trim().to_string()
        }
    } else {
        name_part.trim_end_matches('*').trim().to_string()
    };

    Some(PowerPlan {
        guid,
        name,
        is_active,
    })
}

// 复制电源计划
pub fn duplicate_power_plan(guid: &str) -> Result<String, String> {
    let output = Command::new("powercfg")
        .args(["/duplicatescheme", guid])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if !output.status.success() {
        let (cow, _encoding_used, had_errors) = GBK.decode(&output.stderr);
        if had_errors {
            return Err("GBK 解码失败".to_string());
        }
        return Err(cow.into_owned());
    }

    // 从输出中提取新的 GUID
    let (cow, _encoding_used, had_errors) = GBK.decode(&output.stdout);
    if had_errors {
        return Err("GBK 解码失败".to_string());
    }
    let output_str = cow.into_owned();

    // 尝试从输出中提取 GUID
    if let Some(guid) = output_str
        .lines()
        .find(|line| line.contains("GUID:"))
        .and_then(|line| line.split("GUID:").nth(1))
        .map(|s| s.trim().to_string())
    {
        Ok(guid)
    } else {
        Err("无法从输出中提取 GUID".to_string())
    }
}

// 删除电源计划
pub fn delete_power_plan(guid: &str) -> Result<(), String> {
    // 首先检查是否是当前活动的计划
    let plans = get_power_plans()?;
    if plans.iter().any(|plan| plan.guid == guid && plan.is_active) {
        return Err("不能删除当前活动的电源计划".to_string());
    }

    let output = Command::new("powercfg")
        .args(["/delete", guid])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if !output.status.success() {
        let (cow, _encoding_used, had_errors) = GBK.decode(&output.stderr);
        if had_errors {
            return Err("GBK 解码失败".to_string());
        }
        return Err(cow.into_owned());
    }

    Ok(())
}

// 更改计划名称
pub fn rename_power_plan(guid: &str, new_name: &str) -> Result<(), String> {
    let output = Command::new("powercfg")
        .args(["/changename", guid, new_name])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if !output.status.success() {
        let (cow, _encoding_used, had_errors) = GBK.decode(&output.stderr);
        if had_errors {
            return Err("GBK 解码失败".to_string());
        }
        return Err(cow.into_owned());
    }

    Ok(())
}

// 添加 tauri 命令
#[tauri::command]
pub async fn duplicate_power_plan_command(guid: String) -> Result<String, String> {
    duplicate_power_plan(&guid)
}

#[tauri::command]
pub async fn delete_power_plan_command(guid: String) -> Result<(), String> {
    delete_power_plan(&guid)
}

#[tauri::command]
pub async fn rename_power_plan_command(guid: String, new_name: String) -> Result<(), String> {
    rename_power_plan(&guid, &new_name)
}

// 导出电源计划
pub fn export_power_plan(guid: &str, file_path: &str) -> Result<(), String> {
    let output = Command::new("powercfg")
        .args(["/export", file_path, guid])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("执行导出命令失败: {}", e))?;

    if !output.status.success() {
        let (cow, _encoding_used, had_errors) = GBK.decode(&output.stderr);
        if had_errors {
            return Err("GBK 解码失败".to_string());
        }
        return Err(cow.into_owned());
    }

    Ok(())
}

// 导入电源计划
pub fn import_power_plan(file_path: &str) -> Result<String, String> {
    let output = Command::new("powercfg")
        .args(["/import", file_path])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("执行导入命令失败: {}", e))?;

    if !output.status.success() {
        let (cow, _encoding_used, had_errors) = GBK.decode(&output.stderr);
        if had_errors {
            return Err("GBK 解码失败".to_string());
        }
        return Err(cow.into_owned());
    }

    // 从输出中提取新的 GUID
    let (cow, _encoding_used, had_errors) = GBK.decode(&output.stdout);
    if had_errors {
        return Err("GBK 解码失败".to_string());
    }
    let output_str = cow.into_owned();

    // 尝试从输出中提取 GUID
    if let Some(guid) = output_str
        .lines()
        .find(|line| line.contains("GUID:"))
        .and_then(|line| line.split("GUID:").nth(1))
        .map(|s| s.trim().to_string())
    {
        Ok(guid)
    } else {
        Err("无法从输出中提取 GUID".to_string())
    }
}

#[tauri::command]
pub async fn export_power_plan_command(guid: String, file_path: String) -> Result<(), String> {
    export_power_plan(&guid, &file_path)
}

#[tauri::command]
pub async fn import_power_plan_command(file_path: String) -> Result<String, String> {
    import_power_plan(&file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_power_plans() {
        let sample_output = r#"
现有电源使用方案 (* Active)
-----------------------------------
电源方案 GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (平衡)
电源方案 GUID: 8bd00add-abf1-47cf-98b4-0e38e7999415  (我的自定义计划 1) *
电源方案 GUID: cf285094-3f36-42e5-b53a-d0b098d10d8b  (Customized Clock)
"#;

        let plans = parse_power_plans(sample_output).unwrap();
        assert_eq!(plans.len(), 3);
        assert_eq!(plans[1].is_active, true);
        assert_eq!(plans[1].name, "我的自定义计划 1");
    }
}

// 修改执行命令的函数
fn execute_powercfg(args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("powercfg")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW) // 添加这个标志来隐藏窗口
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
