import React from "react";
import { cn } from "@/utils/cn";

interface LoadingProps {
  size?: "sm" | "md" | "lg" | "xl";
  variant?: "spinner" | "dots" | "pulse";
  className?: string;
  text?: string;
}

export const Loading: React.FC<LoadingProps> = ({
  size = "md",
  variant = "spinner",
  className,
  text,
}) => {
  const sizeClasses = {
    sm: "w-4 h-4",
    md: "w-6 h-6",
    lg: "w-8 h-8",
    xl: "w-12 h-12",
  };

  const textSizeClasses = {
    sm: "text-xs",
    md: "text-sm",
    lg: "text-base",
    xl: "text-lg",
  };

  const renderSpinner = () => (
    <div
      className={cn(
        "animate-spin rounded-full border-2 border-muted border-t-primary",
        sizeClasses[size],
        className
      )}
    />
  );

  const renderDots = () => (
    <div className={cn("flex space-x-1", className)}>
      <div
        className={cn(
          "bg-primary rounded-full animate-bounce",
          size === "sm" ? "w-1 h-1" : size === "md" ? "w-2 h-2" : size === "lg" ? "w-3 h-3" : "w-4 h-4"
        )}
        style={{ animationDelay: "0ms" }}
      />
      <div
        className={cn(
          "bg-primary rounded-full animate-bounce",
          size === "sm" ? "w-1 h-1" : size === "md" ? "w-2 h-2" : size === "lg" ? "w-3 h-3" : "w-4 h-4"
        )}
        style={{ animationDelay: "150ms" }}
      />
      <div
        className={cn(
          "bg-primary rounded-full animate-bounce",
          size === "sm" ? "w-1 h-1" : size === "md" ? "w-2 h-2" : size === "lg" ? "w-3 h-3" : "w-4 h-4"
        )}
        style={{ animationDelay: "300ms" }}
      />
    </div>
  );

  const renderPulse = () => (
    <div
      className={cn(
        "bg-primary rounded-full animate-pulse",
        sizeClasses[size],
        className
      )}
    />
  );

  const renderLoader = () => {
    switch (variant) {
      case "dots":
        return renderDots();
      case "pulse":
        return renderPulse();
      default:
        return renderSpinner();
    }
  };

  if (text) {
    return (
      <div className="flex flex-col items-center space-y-2">
        {renderLoader()}
        <span className={cn("text-muted-foreground", textSizeClasses[size])}>
          {text}
        </span>
      </div>
    );
  }

  return renderLoader();
};

// Specific loading components for common use cases
export const PageLoading: React.FC<{ text?: string }> = ({ text = "Loading..." }) => (
  <div className="flex items-center justify-center min-h-[200px]">
    <Loading size="lg" text={text} />
  </div>
);

export const ButtonLoading: React.FC = () => (
  <Loading size="sm" variant="spinner" className="text-current" />
);

export const InlineLoading: React.FC<{ text?: string }> = ({ text }) => (
  <div className="flex items-center space-x-2">
    <Loading size="sm" />
    {text && <span className="text-sm text-muted-foreground">{text}</span>}
  </div>
);

// Full screen loading overlay
export const FullScreenLoading: React.FC<{ text?: string }> = ({ 
  text = "Processing..." 
}) => (
  <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm">
    <div className="flex flex-col items-center space-y-4 p-8 bg-card rounded-lg shadow-lg border">
      <Loading size="xl" />
      <p className="text-lg font-medium">{text}</p>
    </div>
  </div>
); 