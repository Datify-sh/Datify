import { AuthLayout } from "@/components/layout/auth-layout";
import { DashboardLayout } from "@/components/layout/dashboard-layout";
import { Spinner } from "@/components/ui/spinner";
import { AuthProvider } from "@/contexts/auth-context";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "next-themes";
import * as React from "react";
import { BrowserRouter, Route, Routes } from "react-router-dom";
import { Toaster } from "sonner";

const loadAuditLogsPage = () => import("@/pages/audit-logs");
const loadLoginPage = () => import("@/pages/auth/login");
const loadRegisterPage = () => import("@/pages/auth/register");
const loadDashboardPage = () => import("@/pages/dashboard");
const loadDatabaseDetailPage = () => import("@/pages/databases/detail");
const loadDatabaseConnectionTab = () => import("@/pages/databases/tabs/connection");
const loadDatabaseEditorTab = () => import("@/pages/databases/tabs/editor");
const loadDatabaseBranchesTab = () => import("@/pages/databases/tabs/branches");
const loadDatabaseMetricsTab = () => import("@/pages/databases/tabs/metrics");
const loadDatabaseTerminalTab = () => import("@/pages/databases/tabs/terminal");
const loadDatabaseLogsTab = () => import("@/pages/databases/tabs/logs");
const loadDatabaseConfigTab = () => import("@/pages/databases/tabs/config");
const loadDatabaseSettingsTab = () => import("@/pages/databases/tabs/settings");
const loadProjectDetailPage = () => import("@/pages/projects/detail");
const loadProjectsListPage = () => import("@/pages/projects/list");

const AuditLogsPage = React.lazy(() =>
  loadAuditLogsPage().then((module) => ({ default: module.AuditLogsPage })),
);
const LoginPage = React.lazy(() =>
  loadLoginPage().then((module) => ({ default: module.LoginPage })),
);
const RegisterPage = React.lazy(() =>
  loadRegisterPage().then((module) => ({ default: module.RegisterPage })),
);
const DashboardPage = React.lazy(() =>
  loadDashboardPage().then((module) => ({ default: module.DashboardPage })),
);
const DatabaseDetailPage = React.lazy(() =>
  loadDatabaseDetailPage().then((module) => ({ default: module.DatabaseDetailPage })),
);
const DatabaseConnectionTab = React.lazy(() =>
  loadDatabaseConnectionTab().then((module) => ({ default: module.DatabaseConnectionTab })),
);
const DatabaseEditorTab = React.lazy(() =>
  loadDatabaseEditorTab().then((module) => ({ default: module.DatabaseEditorTab })),
);
const DatabaseBranchesTab = React.lazy(() =>
  loadDatabaseBranchesTab().then((module) => ({ default: module.DatabaseBranchesTab })),
);
const DatabaseMetricsTab = React.lazy(() =>
  loadDatabaseMetricsTab().then((module) => ({ default: module.DatabaseMetricsTab })),
);
const DatabaseTerminalTab = React.lazy(() =>
  loadDatabaseTerminalTab().then((module) => ({ default: module.DatabaseTerminalTab })),
);
const DatabaseLogsTab = React.lazy(() =>
  loadDatabaseLogsTab().then((module) => ({ default: module.DatabaseLogsTab })),
);
const DatabaseConfigTab = React.lazy(() =>
  loadDatabaseConfigTab().then((module) => ({ default: module.DatabaseConfigTab })),
);
const DatabaseSettingsTab = React.lazy(() =>
  loadDatabaseSettingsTab().then((module) => ({ default: module.DatabaseSettingsTab })),
);
const ProjectDetailPage = React.lazy(() =>
  loadProjectDetailPage().then((module) => ({ default: module.ProjectDetailPage })),
);
const ProjectsListPage = React.lazy(() =>
  loadProjectsListPage().then((module) => ({ default: module.ProjectsListPage })),
);

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 30,
      gcTime: 1000 * 60 * 5,
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function RouteFallback() {
  return (
    <div className="flex items-center justify-center py-16 text-sm text-muted-foreground">
      <Spinner className="size-5" />
      <span className="ml-2">Loading page...</span>
    </div>
  );
}

function RouteSuspense({ children }: { children: React.ReactNode }) {
  return <React.Suspense fallback={<RouteFallback />}>{children}</React.Suspense>;
}

export function App() {
  return (
    <ThemeProvider attribute="class" defaultTheme="system" enableSystem={true}>
      <QueryClientProvider client={queryClient}>
        <BrowserRouter>
          <AuthProvider>
            <Routes>
              {/* Auth routes */}
              <Route element={<AuthLayout />}>
                <Route
                  path="/login"
                  element={
                    <RouteSuspense>
                      <LoginPage />
                    </RouteSuspense>
                  }
                />
                <Route
                  path="/register"
                  element={
                    <RouteSuspense>
                      <RegisterPage />
                    </RouteSuspense>
                  }
                />
              </Route>

              {/* Dashboard routes */}
              <Route element={<DashboardLayout />}>
                <Route
                  path="/"
                  element={
                    <RouteSuspense>
                      <DashboardPage />
                    </RouteSuspense>
                  }
                />
                <Route
                  path="/projects"
                  element={
                    <RouteSuspense>
                      <ProjectsListPage />
                    </RouteSuspense>
                  }
                />
                <Route
                  path="/projects/:id"
                  element={
                    <RouteSuspense>
                      <ProjectDetailPage />
                    </RouteSuspense>
                  }
                />
                <Route
                  path="/databases/:id"
                  element={
                    <RouteSuspense>
                      <DatabaseDetailPage />
                    </RouteSuspense>
                  }
                >
                  <Route
                    index
                    element={
                      <RouteSuspense>
                        <DatabaseConnectionTab />
                      </RouteSuspense>
                    }
                  />
                  <Route
                    path="editor"
                    element={
                      <RouteSuspense>
                        <DatabaseEditorTab />
                      </RouteSuspense>
                    }
                  />
                  <Route
                    path="branches"
                    element={
                      <RouteSuspense>
                        <DatabaseBranchesTab />
                      </RouteSuspense>
                    }
                  />
                  <Route
                    path="metrics"
                    element={
                      <RouteSuspense>
                        <DatabaseMetricsTab />
                      </RouteSuspense>
                    }
                  />
                  <Route
                    path="terminal"
                    element={
                      <RouteSuspense>
                        <DatabaseTerminalTab />
                      </RouteSuspense>
                    }
                  />
                  <Route
                    path="logs"
                    element={
                      <RouteSuspense>
                        <DatabaseLogsTab />
                      </RouteSuspense>
                    }
                  />
                  <Route
                    path="config"
                    element={
                      <RouteSuspense>
                        <DatabaseConfigTab />
                      </RouteSuspense>
                    }
                  />
                  <Route
                    path="settings"
                    element={
                      <RouteSuspense>
                        <DatabaseSettingsTab />
                      </RouteSuspense>
                    }
                  />
                </Route>
                <Route
                  path="/audit-logs"
                  element={
                    <RouteSuspense>
                      <AuditLogsPage />
                    </RouteSuspense>
                  }
                />
              </Route>
            </Routes>
            <Toaster position="bottom-right" />
          </AuthProvider>
        </BrowserRouter>
      </QueryClientProvider>
    </ThemeProvider>
  );
}

export default App;
