//! Explicitly authorized, unattended hardware experiment. Never part of a
//! normal gate: it temporarily changes AC sleep and allows real S3 sleep.

use super::{FakeClaude, claim_file, scratch};

#[test]
#[ignore = "manual: needs unattended sleep permission and PGG_AWAKE_PHYSICAL_LOG"]
fn a_desktop_wait_allows_physical_sleep_after_a_protected_turn() {
    use crate::awake::{Mark, apply};
    let report = std::env::var("PGG_AWAKE_PHYSICAL_LOG").expect("an explicit durable report path");
    let dir = scratch("physical-desktop");
    let log = dir.join("main.log");
    std::fs::write(&log, "").unwrap();
    std::fs::write(
        dir.join("desktop-log.path"),
        log.to_string_lossy().as_bytes(),
    )
    .unwrap();
    std::fs::remove_file(dir.join(crate::awake::DESKTOP_OFF)).unwrap();
    let claude = FakeClaude::start(&dir);
    apply(&dir, &claim_file("s"), &claude.hook(), Mark::Prompt).expect("a turn starts");
    apply(
        &dir,
        &claim_file("s"),
        &claude.hook(),
        Mark::ToolStart {
            id: "physical-browser",
            name: "mcp__Claude_Browser__navigate",
            input: "fixture",
        },
    )
    .expect("a childless call starts");
    assert_eq!(super::next_reading(&dir), super::holding());
    let script = format!(
        "$report='{}'\n$permissionLog='{}'\n{}",
        report.replace('\'', "''"),
        log.display().to_string().replace('\'', "''"),
        PROBE
    );
    let path = dir.join("physical.ps1");
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend_from_slice(script.as_bytes());
    std::fs::write(&path, bytes).unwrap();
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-File"])
        .arg(path)
        .output()
        .expect("the physical observer runs");
    assert!(
        output.status.success(),
        "observer failed; report {report}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(super::next_reading(&dir), super::released());
    let evidence = std::fs::read_to_string(&report).expect("durable hardware evidence");
    assert!(evidence.contains("RESULT pass"), "{evidence}");
    eprintln!("Physical sleep verified; report {report}");
}

// This observer makes no execution-state request and sends no input. Its
// hidden window only receives display/power notifications. The wake timer
// bounds the unattended experiment without forcing a sleep transition.
const PROBE: &str = r#"
$ErrorActionPreference='Stop'
Add-Type -ReferencedAssemblies System.Windows.Forms -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public sealed class PhysicalAwakeProbe : NativeWindow, IDisposable {
  [StructLayout(LayoutKind.Sequential)] struct LastInput { public uint Size, Tick; }
  [DllImport("user32.dll")] static extern bool GetLastInputInfo(ref LastInput info);
  [DllImport("user32.dll")] static extern IntPtr RegisterPowerSettingNotification(IntPtr window, ref Guid setting, uint flags);
  [DllImport("user32.dll")] static extern bool UnregisterPowerSettingNotification(IntPtr handle);
  [DllImport("powrprof.dll")] static extern uint CallNtPowerInformation(int level, IntPtr input, uint size, out uint state, uint outputSize);
  [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr CreateWaitableTimer(IntPtr attrs, bool manual, string name);
  [DllImport("kernel32.dll", SetLastError=true)] static extern bool SetWaitableTimer(IntPtr timer, ref long due, int period, IntPtr callback, IntPtr data, bool resume);
  [DllImport("kernel32.dll")] static extern bool CancelWaitableTimer(IntPtr timer);
  [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
  readonly string report, permissionLog;
  readonly ApplicationContext context=new ApplicationContext();
  readonly Timer sampler=new Timer();
  readonly DateTime start=DateTime.UtcNow;
  IntPtr notification, wake;
  bool released, offDuringWork, earlySleep, sleptAfterRelease, lostHold;
  int sample;
  void Record(string text) { File.AppendAllText(report, DateTime.UtcNow.ToString("o")+" "+text+Environment.NewLine); }
  PhysicalAwakeProbe(string report, string permissionLog) {
    this.report=report; this.permissionLog=permissionLog;
    CreateHandle(new CreateParams { Caption="Awake hardware observer" });
    var setting=new Guid("6fe69556-704a-47a0-8f24-c28d936fda47");
    notification=RegisterPowerSettingNotification(Handle,ref setting,0);
    if(notification==IntPtr.Zero) throw new InvalidOperationException("Display observation unavailable");
    wake=CreateWaitableTimer(IntPtr.Zero,true,null);
    long due=-480L*10000000L;
    if(wake==IntPtr.Zero || !SetWaitableTimer(wake,ref due,0,IntPtr.Zero,IntPtr.Zero,true))
      throw new InvalidOperationException("Wake timer unavailable: "+Marshal.GetLastWin32Error());
    sampler.Interval=1000;
    sampler.Tick+=Tick;
    sampler.Start();
    Record("START protected=210s wake=480s");
  }
  void Tick(object sender, EventArgs args) {
    var elapsed=(DateTime.UtcNow-start).TotalSeconds;
    if(!released && elapsed>=210) {
      var stamp=DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss");
      File.AppendAllText(permissionLog, stamp+" [info] Mapping internal session local_physical to CLI session s\n"+
        stamp+" [info] Emitted tool permission request physical-request for browser:open_file in session local_physical\n");
      released=true;
      Record("WAIT permission log emitted");
    }
    if(sample++%5==0) {
      var input=new LastInput {Size=8}; GetLastInputInfo(ref input);
      uint state; var result=CallNtPowerInformation(16,IntPtr.Zero,0,out state,4);
      Record("SAMPLE idleSeconds="+(unchecked((uint)Environment.TickCount-input.Tick)/1000)+" state="+state+" status="+result);
      if(!released && (result!=0 || (state&1)==0)) lostHold=true;
    }
    if(elapsed>=480) { sampler.Stop(); context.ExitThread(); }
  }
  protected override void WndProc(ref Message message) {
    if(message.Msg==0x218) {
      int kind=message.WParam.ToInt32();
      if(kind==4) {
        Record("SUSPEND released="+released);
        if(released) sleptAfterRelease=true; else earlySleep=true;
      }
      if(kind==7 || kind==18) Record("RESUME kind="+kind);
      if(kind==0x8013) {
        int state=Marshal.ReadInt32(message.LParam,20);
        Record("DISPLAY state="+state+" released="+released);
        if(state==0 && !released) offDuringWork=true;
      }
    }
    base.WndProc(ref message);
  }
  public void Dispose() {
    sampler.Dispose();
    if(notification!=IntPtr.Zero) UnregisterPowerSettingNotification(notification);
    if(wake!=IntPtr.Zero) { CancelWaitableTimer(wake); CloseHandle(wake); }
    DestroyHandle(); context.Dispose();
  }
  public static bool Run(string report, string permissionLog) {
    Application.SetUnhandledExceptionMode(UnhandledExceptionMode.ThrowException);
    using(var probe=new PhysicalAwakeProbe(report,permissionLog)) {
      Application.Run(probe.context);
      bool pass=probe.offDuringWork && !probe.earlySleep && !probe.lostHold && probe.sleptAfterRelease;
      probe.Record("RESULT "+(pass?"pass":"inconclusive")+" displayOff="+probe.offDuringWork+
        " earlySleep="+probe.earlySleep+" lostHold="+probe.lostHold+" sleptAfterRelease="+probe.sleptAfterRelease);
      return pass;
    }
  }
}
'@
function Active-Scheme {
  $text=powercfg /getactivescheme | Out-String
  if ($LASTEXITCODE -ne 0) { throw 'Cannot read power scheme' }
  return [regex]::Match($text,'[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}').Value
}
$scheme=Active-Scheme
if (-not $scheme) { throw 'No active power scheme' }
$settings=powercfg /query $scheme SUB_SLEEP STANDBYIDLE | Out-String
$matched=[regex]::Match($settings,'(?m)^.*AC.*:\s*0x([0-9a-fA-F]+)\s*$')
if (-not $matched.Success) { throw 'Cannot read the original AC sleep timeout' }
$original=[Convert]::ToUInt32($matched.Groups[1].Value,16)
$start=Get-Date
[IO.File]::WriteAllText($report, "PLAN $scheme AC-seconds=$original"+[Environment]::NewLine)
$pass=$false
try {
  powercfg /setacvalueindex $scheme SUB_SLEEP STANDBYIDLE 180
  if ($LASTEXITCODE -ne 0) { throw 'Cannot set the temporary timeout' }
  powercfg /setactive $scheme
  if ($LASTEXITCODE -ne 0) { throw 'Cannot apply the temporary timeout' }
  $pass=[PhysicalAwakeProbe]::Run($report,$permissionLog)
} finally {
  powercfg /setacvalueindex $scheme SUB_SLEEP STANDBYIDLE $original
  if ($LASTEXITCODE -ne 0) { throw 'RESTORE FAILED: original AC timeout' }
  if ((Active-Scheme) -eq $scheme) {
    powercfg /setactive $scheme
    if ($LASTEXITCODE -ne 0) { throw 'RESTORE FAILED: active scheme' }
  }
  [IO.File]::AppendAllText($report, "RESTORED AC-seconds=$original"+[Environment]::NewLine)
}
Get-WinEvent -FilterHashtable @{LogName='System';ProviderName='Microsoft-Windows-Kernel-Power';Id=42;StartTime=$start} -ErrorAction SilentlyContinue |
  ForEach-Object { [IO.File]::AppendAllText($report, "SYSTEM-EVENT "+$_.TimeCreated.ToString('o')+" id="+$_.Id+[Environment]::NewLine) }
if (-not $pass) { throw "Physical result inconclusive; see $report" }
"#;
