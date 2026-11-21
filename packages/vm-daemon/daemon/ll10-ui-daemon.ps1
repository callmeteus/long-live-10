# LL10 UI daemon - Long Live 10 VM automation
#
# Runs INSIDE the Windows 10 guest in the interactive console session (session 1),
# started from the user's Startup folder. WinRM cannot inject keys into the
# desktop (session 0), but this daemon can.
#
# Listens on TCP :::55990. A VirtualBox NAT rule maps host 127.0.0.1:55990 to
# the guest, so the host controller (client/vmctl.ps1) talks to it directly.
# No WinRM is needed to drive the UI or read logs.
#
# COMMANDS (one line per connection):
#   PING, FOCUS, WIN, TEXT <value>, KEY <vk>, ESC, ENTER, SNAP, PROCS,
#   LONGRUN [n]   run n Start-search iterations, capture state on failure
#   GETLOG        return the reproduction log
#   STATUS
#
# Logs: C:\ll10\ui-daemon.log, C:\ll10\repro-long.log
param([int]$Port = 55990)

$ErrorActionPreference = "Continue"
$DL = "C:\ll10\ui-daemon.log"
$RL_PATH = "C:\ll10\repro-long.log"
function L { param($m) Add-Content -Path $DL -Value "$(Get-Date -Format o) $m" -Encoding UTF8 }
function RL { param($m) Add-Content -Path $RL_PATH -Value "$(Get-Date -Format o) $m" -Encoding UTF8 }
New-Item -ItemType Directory -Path "C:\ll10" -Force | Out-Null
Set-Content -Path $DL -Value "daemon v2 starting $(Get-Date -Format o)" -Encoding UTF8

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class VmNative2 {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string name);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
}
"@

function Get-FgInfo {
    $fg = [VmNative2]::GetForegroundWindow(); $p = [uint32]0
    [VmNative2]::GetWindowThreadProcessId($fg, [ref]$p) | Out-Null
    return "fg=$fg pid=$p"
}
function Invoke-KeyVk { param([byte]$Vk)
    [VmNative2]::keybd_event($Vk,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 40
    [VmNative2]::keybd_event($Vk,0,0x2,[UIntPtr]::Zero) }
function Focus-Desktop {
    $pm = [VmNative2]::FindWindow("Progman",$null)
    if ($pm -ne [IntPtr]::Zero) { [VmNative2]::ShowWindow($pm,5)|Out-Null; [VmNative2]::SetForegroundWindow($pm)|Out-Null }
    $tray = [VmNative2]::FindWindow("Shell_TrayWnd",$null)
    if ($tray -ne [IntPtr]::Zero) { [VmNative2]::SetForegroundWindow($tray)|Out-Null }
    return "pm=$pm tray=$tray $(Get-FgInfo)"
}
function Dump-Windows {
    $root=[System.Windows.Automation.AutomationElement]::RootElement
    $all=$root.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)
    $out="count=$($all.Count)"
    foreach($w in $all){$n=$w.Current.Name;$c=$w.Current.ClassName;$p=$w.Current.ProcessId;if($n-or$c){$out+=" | [$n] class=[$c] pid=$p"}}
    return $out
}
function Invoke-LongRun { param([int]$N=60)
    Set-Content -Path $RL_PATH -Value "longrun start $(Get-Date -Format o) n=$N" -Encoding UTF8
    $fails=0
    for($i=1;$i-le$N;$i++){
        RL "--- iteration $i ---"
        Focus-Desktop|Out-Null; Invoke-KeyVk -Vk 0x5B; Start-Sleep -Milliseconds 400
        [System.Windows.Forms.SendKeys]::SendWait("notepad"); Start-Sleep -Milliseconds 700
        Invoke-KeyVk -Vk 0x0D; Start-Sleep -Milliseconds 600
        $np=Get-Process notepad -ErrorAction SilentlyContinue
        if($np){RL "RESULT ok";$np|Stop-Process -Force -ErrorAction SilentlyContinue}
        else{
            $fails++; RL "RESULT FAIL *** BUG REPRODUCED ***"; RL "SNAP: $(Dump-Windows)"
            $sa=Get-Process SearchApp -ErrorAction SilentlyContinue|Select-Object -First 1
            if($sa){RL "SEARCHAPP pid=$($sa.Id) cpu=$($sa.CPU) threads=$($sa.Threads.Count)"; $dump="C:\ll10\searchapp-fail-$i.dmp"; & rundll32.exe C:\Windows\System32\comsvcs.dll,MiniDump $sa.Id $dump full 2>&1|Out-Null; if(Test-Path $dump){RL "DUMP saved: $dump"}}
            $ev=Get-WinEvent -FilterHashtable @{LogName="Application";StartTime=(Get-Date).AddMinutes(-1)} -ErrorAction SilentlyContinue|Select-Object -First 8 TimeCreated,Id,ProviderName,Message
            foreach($e in $ev){$m=$e.Message;if($m.Length-gt150){$m=$m.Substring(0,150)};RL "EV $($e.TimeCreated) id=$($e.Id) $($e.ProviderName) $m"}
            $tw=Get-WinEvent -LogName "Microsoft-Windows-TWinUI/Operational" -ErrorAction SilentlyContinue|Select-Object -First 5 TimeCreated,Id,Message
            foreach($e in $tw){$m=$e.Message;if($m.Length-gt150){$m=$m.Substring(0,150)};RL "TWINUI $($e.TimeCreated) id=$($e.Id) $m"}
        }
        Invoke-KeyVk -Vk 0x1B; Start-Sleep -Milliseconds 300
    }
    RL "=== DONE failures=$fails / $N ==="
    return "longrun-done failures=$fails / $N"
}
function Execute-Command { param([string]$Line)
    if(-not$Line){return "PONG $(Get-FgInfo)"}
    $parts=$Line -split ' ',2; $cmd=$parts[0].ToUpper(); $arg=if($parts.Count-gt1){$parts[1]}else{""}
    L "cmd: $Line"
    switch($cmd){
        "PING"{return "PONG $(Get-FgInfo)"}
        "FOCUS"{return Focus-Desktop}
        "KEY"{$vk=[Convert]::ToByte($arg,0);Invoke-KeyVk -Vk $vk;return "key-sent vk=$arg $(Get-FgInfo)"}
        "TEXT"{[System.Windows.Forms.SendKeys]::SendWait($arg);return "text-sent [$arg] $(Get-FgInfo)"}
        "WIN"{Focus-Desktop|Out-Null;Invoke-KeyVk -Vk 0x5B;return "win-sent $(Get-FgInfo)"}
        "ESC"{Invoke-KeyVk -Vk 0x1B;return "esc-sent $(Get-FgInfo)"}
        "ENTER"{Invoke-KeyVk -Vk 0x0D;return "enter-sent $(Get-FgInfo)"}
        "SNAP"{return Dump-Windows}
        "PROCS"{return ((Get-Process|Where-Object{$_.ProcessName-match"Search|StartMenu|explorer|notepad"}|ForEach-Object{"$($_.ProcessName):$($_.Id):cpu=$($_.CPU)"}) -join " | ")}
        "STATUS"{return "running $(Get-FgInfo)"}
        "GETLOG"{if(Test-Path$RL_PATH){return (Get-Content$RL_PATH -Raw)}else{return "no log yet"}}
        "LONGRUN"{$n=60;if($arg-match'^\d+$'){$n=[int]$arg};return Invoke-LongRun -N $n}
        default{return "UNKNOWN [$cmd]"}
    }
}

$listener=New-Object System.Net.Sockets.TcpListener([System.Net.IPAddress]::Any,$Port)
$listener.Start(); L "listener ready on $Port"
while($true){
    $client=$null
    try{$client=$listener.AcceptTcpClient()}catch{Start-Sleep -Milliseconds 100;continue}
    try{
        $st=$client.GetStream(); $r=New-Object System.IO.StreamReader($st); $w=New-Object System.IO.StreamWriter($st); $w.AutoFlush=$true
        $line=$r.ReadLine(); if(-not$line){$line="PING"}
        $resp=Execute-Command -Line $line
        $w.WriteLine($resp); L "resp: $resp"
    }catch{L "client error: $($_.Exception.Message)"}finally{if($client){$client.Close()}}
}
