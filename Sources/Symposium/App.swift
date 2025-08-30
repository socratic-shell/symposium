import SwiftUI
import AppKit

@main
struct SymposiumApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 1000, idealWidth: 1200, maxWidth: .infinity,
                       minHeight: 700, idealHeight: 800, maxHeight: .infinity)
        }
    }
}