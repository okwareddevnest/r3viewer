import React from "react";
import { cn } from "@/utils/cn";

interface BadgeProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: "default" | "secondary" | "success" | "warning" | "destructive" | "outline";
  size?: "sm" | "md" | "lg";
  children: React.ReactNode;
}

const Badge = React.forwardRef<HTMLDivElement, BadgeProps>(
  ({ className, variant = "default", size = "md", children, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          "inline-flex items-center rounded-full border font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
          {
            // Variants
            "border-transparent bg-primary text-primary-foreground": variant === "default",
            "border-transparent bg-secondary text-secondary-foreground": variant === "secondary",
            "border-transparent bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300": variant === "success",
            "border-transparent bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300": variant === "warning",
            "border-transparent bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300": variant === "destructive",
            "text-foreground": variant === "outline",
          },
          {
            // Sizes
            "px-2.5 py-0.5 text-xs": size === "sm",
            "px-3 py-1 text-sm": size === "md",
            "px-4 py-1.5 text-base": size === "lg",
          },
          className
        )}
        {...props}
      >
        {children}
      </div>
    );
  }
);

Badge.displayName = "Badge";

// Specific status badges for common use cases
export const StatusBadge: React.FC<{
  status: "pending" | "in-progress" | "completed" | "failed" | "cancelled";
  size?: "sm" | "md" | "lg";
}> = ({ status, size = "md" }) => {
  const statusConfig = {
    pending: { variant: "secondary" as const, label: "Pending", icon: "⏳" },
    "in-progress": { variant: "warning" as const, label: "In Progress", icon: "🔄" },
    completed: { variant: "success" as const, label: "Completed", icon: "✅" },
    failed: { variant: "destructive" as const, label: "Failed", icon: "❌" },
    cancelled: { variant: "outline" as const, label: "Cancelled", icon: "⏹️" },
  };

  const config = statusConfig[status];

  return (
    <Badge variant={config.variant} size={size}>
      <span className="mr-1">{config.icon}</span>
      {config.label}
    </Badge>
  );
};

export const TechStackBadge: React.FC<{
  technology: string;
  size?: "sm" | "md" | "lg";
}> = ({ technology, size = "sm" }) => {
  // Define colors for different technologies
  const getTechColor = (tech: string) => {
    const techLower = tech.toLowerCase();
    if (techLower.includes("react") || techLower.includes("js") || techLower.includes("javascript")) {
      return "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300";
    }
    if (techLower.includes("python") || techLower.includes("django") || techLower.includes("flask")) {
      return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300";
    }
    if (techLower.includes("java") || techLower.includes("spring")) {
      return "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-300";
    }
    if (techLower.includes("typescript") || techLower.includes("ts")) {
      return "bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-300";
    }
    if (techLower.includes("rust") || techLower.includes("go")) {
      return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300";
    }
    // Default color
    return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-300";
  };

  return (
    <div
      className={cn(
        "inline-flex items-center rounded-full border-transparent font-medium",
        getTechColor(technology),
        {
          "px-2 py-0.5 text-xs": size === "sm",
          "px-2.5 py-1 text-sm": size === "md",
          "px-3 py-1.5 text-base": size === "lg",
        }
      )}
    >
      {technology}
    </div>
  );
};

export const ScoreBadge: React.FC<{
  score: number;
  total?: number;
  size?: "sm" | "md" | "lg";
}> = ({ score, total = 100, size = "md" }) => {
  const percentage = total > 0 ? Math.round((score / total) * 100) : 0;
  
  const getScoreVariant = (percentage: number) => {
    if (percentage >= 90) return "success";
    if (percentage >= 70) return "warning";
    if (percentage >= 50) return "secondary";
    return "destructive";
  };

  return (
    <Badge variant={getScoreVariant(percentage)} size={size}>
      {percentage}%
    </Badge>
  );
};

export const ProjectTypeBadge: React.FC<{
  type: "individual" | "team" | "hackathon" | "assignment";
  size?: "sm" | "md" | "lg";
}> = ({ type, size = "sm" }) => {
  const typeConfig = {
    individual: { variant: "default" as const, label: "Individual", icon: "👤" },
    team: { variant: "secondary" as const, label: "Team", icon: "👥" },
    hackathon: { variant: "warning" as const, label: "Hackathon", icon: "🏆" },
    assignment: { variant: "outline" as const, label: "Assignment", icon: "📝" },
  };

  const config = typeConfig[type];

  return (
    <Badge variant={config.variant} size={size}>
      <span className="mr-1">{config.icon}</span>
      {config.label}
    </Badge>
  );
};

export { Badge }; 