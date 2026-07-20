/// Discord Rich Presence has been removed from this build.
///
/// This is a no-op stub that preserves the previous `DiscordGuard` API so all
/// call sites keep compiling, but it never connects to Discord IPC and never
/// publishes any activity.
pub struct DiscordGuard;

impl DiscordGuard {
    pub fn init() -> crate::Result<DiscordGuard> {
        Ok(DiscordGuard)
    }

    pub async fn set_activity(
        &self,
        _msg: &str,
        _reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        Ok(())
    }

    pub async fn force_set_activity(
        &self,
        _msg: &str,
        _reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        Ok(())
    }

    pub async fn clear_activity(
        &self,
        _reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        Ok(())
    }

    pub async fn clear_to_default(
        &self,
        _reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        Ok(())
    }
}
