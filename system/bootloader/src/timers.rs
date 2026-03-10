use {acpi::table::PmTimerInfo, log::debug, spinlocks::once::Once};

mod pit;
mod pm;

static PM_INFO: Once<Option<PmTimerInfo>> = Once::new();

pub fn init_pm(pm_timer_info: Option<PmTimerInfo>) {
    if pm_timer_info.is_some() {
        debug!("PM Timer found");
    } else {
        debug!("PM Timer not found");
    }
    PM_INFO.init(|| pm_timer_info);
}

/// # Note
/// If called from more than one hart at a time, the sleep time will be very innacurate for the second hart
pub fn sleep_us(t: usize) {
    let Some(pm_timer_info) = PM_INFO.poll() else {
        return pit::sleep_us(t);
    };
    let Some(pm_timer_info) = pm_timer_info else {
        return pit::sleep_us(t);
    };

    unsafe { pm::sleep_us(t, pm_timer_info) };
}
