import SwiftUI

struct SplashView: View {
    @EnvironmentObject var permissionManager: PermissionManager
    @EnvironmentObject var settingsManager: SettingsManager
    @EnvironmentObject var agentManager: AgentManager
    @EnvironmentObject var appDelegate: AppDelegate
    
    @State private var currentMessageIndex = 0
    @State private var animationTimer: Timer?
    
    private let loadingMessages = [
        "Warming up the symposium...",
        "Checking permissions...",
        "Scanning for AI agents...",
        "Preparing your workspace...",
        "Almost ready..."
    ]

    var body: some View {
        VStack(spacing: 24) {
            // Logo/Icon
            Image(systemName: "folder.badge.gearshape")
                .font(.system(size: 64))
                .foregroundColor(.blue)
                .scaleEffect(1.0)
                .animation(.easeInOut(duration: 2.0).repeatForever(autoreverses: true), value: currentMessageIndex)

            // App name
            Text("Symposium")
                .font(.largeTitle)
                .fontWeight(.bold)

            // Loading message
            Text(loadingMessages[currentMessageIndex])
                .font(.headline)
                .foregroundColor(.secondary)
                .transition(.opacity)
                .animation(.easeInOut(duration: 0.5), value: currentMessageIndex)

            // Progress indicator
            ProgressView()
                .scaleEffect(0.8)
        }
        .frame(width: 400, height: 300)
        .onAppear {
            startLoadingAnimation()
        }
        .onDisappear {
            stopLoadingAnimation()
        }
    }
    
    private func startLoadingAnimation() {
        animationTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in
            withAnimation {
                currentMessageIndex = (currentMessageIndex + 1) % loadingMessages.count
            }
        }
    }
    
    private func stopLoadingAnimation() {
        animationTimer?.invalidate()
        animationTimer = nil
    }
}
