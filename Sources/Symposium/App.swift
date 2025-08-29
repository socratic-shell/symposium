import SwiftUI
import AppKit

@main
struct SymposiumApp: App {
    @StateObject private var windowManager = WindowManager()
    
    var body: some Scene {
        WindowGroup {
            ContentView(windowManager: windowManager)
                .frame(width: 450, height: 800)
        }
    }
}