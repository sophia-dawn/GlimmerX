import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { AccountWizard } from "@/components/accounts/AccountWizard";

export function OnboardingPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [wizardOpen, setWizardOpen] = useState(true);

  const handleSkip = () => {
    navigate("/");
  };

  const handleWizardSuccess = () => {
    navigate("/accounts");
  };

  return (
    <div className="flex h-screen w-full items-center justify-center bg-background">
      <div className="w-full max-w-md text-center">
        <div className="mb-8">
          <h1 className="mb-2 text-3xl font-bold">
            {t("onboarding.welcomeTitle")}
          </h1>
          <p className="text-muted-foreground">{t("onboarding.welcomeDesc")}</p>
        </div>

        <div className="space-y-3">
          <Button
            variant="default"
            className="w-full"
            onClick={() => setWizardOpen(true)}
          >
            {t("onboarding.addFirstAccount")}
          </Button>
          <Button variant="outline" className="w-full" onClick={handleSkip}>
            {t("onboarding.skipForNow")}
          </Button>
        </div>

        <AccountWizard
          open={wizardOpen}
          onOpenChange={setWizardOpen}
          onSuccess={handleWizardSuccess}
        />
      </div>
    </div>
  );
}
