import { Route, Routes } from "react-router-dom";
import { Sidebar } from "./components/Sidebar";
import { SessionProvider } from "./context/SessionContext";
import { ThemeProvider } from "./context/ThemeContext";
import { ComparePage } from "./pages/ComparePage";
import { Dashboard } from "./pages/Dashboard";
import { DependencyGraphPage } from "./pages/DependencyGraphPage";
import { DriverExplorer } from "./pages/DriverExplorer";
import { FileActivityPage } from "./pages/FileActivityPage";
import { NetworkActivityPage } from "./pages/NetworkActivityPage";
import { ProcessExplorer } from "./pages/ProcessExplorer";
import { ServiceExplorer } from "./pages/ServiceExplorer";
import { StartupSourcesPage } from "./pages/StartupSourcesPage";
import { Timeline } from "./pages/Timeline";

export default function App() {
  return (
    <ThemeProvider>
      <SessionProvider>
        <div className="app-shell">
          <Sidebar />
          <main className="main">
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/processes" element={<ProcessExplorer />} />
              <Route path="/services" element={<ServiceExplorer />} />
              <Route path="/drivers" element={<DriverExplorer />} />
              <Route path="/files" element={<FileActivityPage />} />
              <Route path="/network" element={<NetworkActivityPage />} />
              <Route path="/startup-sources" element={<StartupSourcesPage />} />
              <Route path="/timeline" element={<Timeline />} />
              <Route path="/graph" element={<DependencyGraphPage />} />
              <Route path="/compare" element={<ComparePage />} />
            </Routes>
          </main>
        </div>
      </SessionProvider>
    </ThemeProvider>
  );
}
