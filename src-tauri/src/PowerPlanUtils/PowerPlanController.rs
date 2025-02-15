use super::GetPowerPlans::{get_power_plans, enumerate_power_scheme_subgroups, PowerPlan, PowerSubgroup,get_unified_power_scheme_json_by_scheme_guid,check_if_scheme_is_valid};
use windows::{
    core::GUID,
};

/// 对外统一导出的控制器结构体
pub struct PowerPlanController;
// pub struct PowerPlan = GetPowerPlans::PowerPlan;



impl PowerPlanController {
    /// 列出所有电源计划，返回包含 uuid 和 name 的数组
    pub fn list_plans() -> Result<Vec<PowerPlan>, String> {
        get_power_plans()
    }
    pub fn list_subgroups(guid_str: &str) -> Result<Vec<PowerSubgroup>, String> {
        enumerate_power_scheme_subgroups(guid_str)
    }
    pub fn get_power_plans_json_by_scheme_guid(guid_str: &str)->Result<String,String>{
        get_unified_power_scheme_json_by_scheme_guid(guid_str)
    }
    pub fn check_if_scheme_is_valid(guid_str: &str) -> bool {
        check_if_scheme_is_valid(guid_str)
    }
}