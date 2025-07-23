use crate::TargetType;
use crate::models::share::ShareId;
use crate::models::share::role::ShareRole;

#[derive(Clone, Debug, serde::Serialize)]
pub struct ShareMember {
    pub share_id: ShareId,
    pub email: String,
    pub name: String,
    pub is_group_share: bool,
    pub role: ShareRole,
    pub target_type: TargetType,
}
