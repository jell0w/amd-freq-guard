use super::GetPowerPlans::{get_power_plans, PowerPlan};


/// 对外统一导出的控制器结构体
pub struct PowerPlanController;
// pub struct PowerPlan = GetPowerPlans::PowerPlan;


impl PowerPlanController {
    /// 列出所有电源计划，返回包含 uuid 和 name 的数组
    pub fn list_plans() -> Result<Vec<PowerPlan>, String> {
        get_power_plans()
    }
}