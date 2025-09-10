import Foundation
import SwiftUI

class SettingsManager: ObservableObject {
    @AppStorage("selectedAgent") var selectedAgentRaw: String = AgentType.qcli.rawValue
    @AppStorage("activeProjectPath") var activeProjectPath: String = ""
    
    var selectedAgent: AgentType {
        get { AgentType(rawValue: selectedAgentRaw) ?? .qcli }
        set { selectedAgentRaw = newValue.rawValue }
    }
}
