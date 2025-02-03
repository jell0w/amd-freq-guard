use planif::enums::TaskCreationFlags;
use planif::schedule::TaskScheduler;
use planif::schedule_builder::{Action, ScheduleBuilder};
use planif::settings::{Duration, LogonType, PrincipalSettings, RunLevel, Settings};
use planif::task::Task;
use log::{error, info};

const FOLDER: &str = "JellowSoftware";
const TASK_NAME: &str = "AMDFreqGuardAutoStart";

pub fn setup_autostart(exe_path: String) -> Result<(), String> {
    let ts = TaskScheduler::new().map_err(|e| e.to_string())?;
    let com = ts.get_com();
    let sb = ScheduleBuilder::new(&com).map_err(|e| e.to_string())?;

    let mut settings = Settings::new();
    settings.stop_if_going_on_batteries = Some(false);
    settings.disallow_start_if_on_batteries = Some(false);
    settings.enabled = Some(true);

    let user_id = format!("{}\\{}", whoami::devicename(), whoami::username());
    info!("设置自启动用户: {}", user_id);

    let principal_settings = PrincipalSettings {
        display_name: "".to_string(),
        group_id: None,
        id: "".to_string(),
        logon_type: LogonType::InteractiveToken,
        run_level: RunLevel::Highest,
        user_id: Some(user_id),
    };

    let result = sb.create_logon()
        .author(FOLDER).map_err(|e| e.to_string())?
        .trigger("LogonTrigger", true).map_err(|e| e.to_string())?
        //在path=&exe_path后加--autostart，字符串拼接，不要直接用第三个参数
        .action(Action::new(TASK_NAME, &exe_path, "", "--autostart")).map_err(|e| e.to_string())?
        .in_folder(FOLDER).map_err(|e| e.to_string())?
        .principal(principal_settings).map_err(|e| e.to_string())?
        .settings(settings).map_err(|e| e.to_string())?
        .delay(Duration {
            seconds: Some(10),
            ..Default::default()
        }).map_err(|e| e.to_string())?
        .build().map_err(|e| e.to_string())?
        .register(TASK_NAME, TaskCreationFlags::CreateOrUpdate as i32).map_err(|e| e.to_string())?;

    info!("自启动任务设置成功");
    Ok(())
}

pub fn enable_autostart() -> Result<(), String> {
    let _ts = TaskScheduler::new().map_err(|e| e.to_string())?;
    let task = Task::new().map_err(|e| e.to_string())?;
    task.enable(FOLDER, TASK_NAME).map_err(|e| e.to_string())?;
    info!("已启用自启动");
    Ok(())
}

pub fn disable_autostart() -> Result<(), String> {
    let _ts = TaskScheduler::new().map_err(|e| e.to_string())?;
    let task = Task::new().map_err(|e| e.to_string())?;
    task.disable(FOLDER, TASK_NAME).map_err(|e| e.to_string())?;
    info!("已禁用自启动");
    Ok(())
} 