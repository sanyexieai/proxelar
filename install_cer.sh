cd ./proxyapi/src/ca/

rm ./*.cer ./*.key

# 生成私钥
openssl genrsa -out proxelar.key 4096

# 生成根证书，添加必要的扩展属性
openssl req  -x509 -new -nodes -key proxelar.key -sha512 -days 3650 -out proxelar.cer -subj "/CN=proxelar"
# Windows 证书安装（需要 PowerShell）
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
 powershell -Command "Import-Certificate -FilePath .\proxelar.cer -CertStoreLocation Cert:\LocalMachine\Root"
fi