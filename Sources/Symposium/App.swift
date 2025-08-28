import SwiftUI
import AppKit

@main
struct SymposiumApp: App {
    @StateObject private var windowManager = WindowManager()
    
    var body: some Scene {
        WindowGroup {
            ContentView(windowManager: windowManager)
                .frame(width: 400, height: 600)
        }
    }
}