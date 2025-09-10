import Foundation
import SwiftUI

class SettingsManager: ObservableObject {
    @AppStorage("selectedAgent") var selectedAgentRaw: String = AgentType.qcli.rawValue
    @AppStorage("activeProjectPath") var activeProjectPath: String = ""
    
    var selectedAgent: AgentType {
        get { AgentType(rawValue: selectedAgentRaw) ?? .qcli }
        set { selectedAgentRaw = newValue.rawValue }
    }
    
    // Per-project settings stored with project path as key
    private let userDefaults = UserDefaults.standard
    
    func getStackedWindowsEnabled(for projectPath: String) -> Bool {
        return userDefaults.bool(forKey: "stackedWindows_\(projectPath)")
    }
    
    func setStackedWindowsEnabled(_ enabled: Bool, for projectPath: String) {
        userDefaults.set(enabled, forKey: "stackedWindows_\(projectPath)")
    }
}
