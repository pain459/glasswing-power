Name:           glasswing-power
Version:        CHANGEME
Release:        1%{?dist}
Summary:        Power usage / Electricity / Energy monitoring agent 

License:        MIT
URL:            https://github.com/pain459/scaphandre
Source0:        %{name}-%{version}.tar.gz
#Source0 will be github.com url for tar gz of source

BuildRequires:  rust,cargo,openssl-devel,systemd-rpm-macros 
#Requires:

%global debug_package %{nil}

%description

%prep
%autosetup

%build
cargo build --release

%pre

%install
#rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/%{_bindir}/
cp target/release/glasswing-power $RPM_BUILD_ROOT/%{_bindir}/
chmod +x $RPM_BUILD_ROOT/%{_bindir}/glasswing-power
mkdir -p $RPM_BUILD_ROOT/lib/systemd/system
mkdir -p $RPM_BUILD_ROOT/etc/glasswing-power
echo "GLASSWING_POWER_ARGS=prometheus" > $RPM_BUILD_ROOT/etc/glasswing-power/default
mkdir -p $RPM_BUILD_ROOT/lib/systemd/system
cp packaging/linux/redhat/scaphandre.service $RPM_BUILD_ROOT/lib/systemd/system/glasswing-power.service

%post
%systemd_post glasswing-power.service

%preun
%systemd_preun glasswing-power.service

%postun
%systemd_postun_with_restart glasswing-power.service

%clean
#rm -rf $RPM_BUILD_ROOT

%files
#%doc README.md
%{_bindir}/glasswing-power
/lib/systemd/system/glasswing-power.service
/etc/glasswing-power/default

#%license LICENSE

%changelog
