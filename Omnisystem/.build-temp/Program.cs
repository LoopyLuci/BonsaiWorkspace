using System;
using System.Diagnostics;
using System.IO;

namespace Omnisystem {
    class Program {
        static void Main(string[] args) {
            // Get Omnisystem directory
            string omnisystemDir = Path.Combine(
                AppDomain.CurrentDomain.BaseDirectory,
                "..", "Omnisystem"
            );
            string launcherScript = Path.Combine(omnisystemDir, "Omnisystem.Launcher.ps1");

            // Launch PowerShell with the launcher script
            ProcessStartInfo psi = new ProcessStartInfo {
                FileName = "powershell.exe",
                Arguments = $"-NoExit -ExecutionPolicy Bypass -File \"{launcherScript}\"",
                UseShellExecute = false,
                RedirectStandardOutput = false
            };

            try {
                using (Process p = Process.Start(psi)) {
                    p.WaitForExit();
                }
            } catch (Exception ex) {
                Console.WriteLine("ERROR: Failed to launch Omnisystem");
                Console.WriteLine(ex.Message);
                Environment.Exit(1);
            }
        }
    }
}
