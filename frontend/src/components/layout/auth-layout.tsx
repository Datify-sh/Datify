import { Spinner } from "@/components/ui/spinner";
import { useAuth } from "@/contexts/auth-context";
import { Navigate, Outlet } from "react-router-dom";

export function AuthLayout() {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div className="flex h-screen w-full items-center justify-center">
        <Spinner className="size-8" />
      </div>
    );
  }

  if (isAuthenticated) {
    return <Navigate to="/" replace />;
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <div className="w-full max-w-sm space-y-6">
        <Outlet />
      </div>
    </div>
  );
}
