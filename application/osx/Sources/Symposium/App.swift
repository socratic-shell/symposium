import SwiftUI
import AppKit

@main
struct SymposiumApp: App {
    var body: some Scene {
        WindowGroup {
            MainView()
        }
        
        Settings {
            SettingsView()
        }
    }
}