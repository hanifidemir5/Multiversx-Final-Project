
#![no_std]

multiversx_sc::imports!();

multiversx_sc::derive_imports!();

#[derive(

    multiversx_sc::codec::derive::TopEncode,
    multiversx_sc::codec::derive::TopDecode,
    

    multiversx_sc::codec::derive::NestedEncode, 
    multiversx_sc::codec::derive::NestedDecode,

    multiversx_sc::derive::TypeAbi,
    

    PartialEq,

    Clone
)]

#[derive(Debug)]
pub enum OfferStatus {

    Active,

    Completed,

    Cancelled
}

#[derive(

    multiversx_sc::codec::derive::TopEncode,
    multiversx_sc::codec::derive::TopDecode,
    
    multiversx_sc::codec::derive::NestedEncode,
    multiversx_sc::codec::derive::NestedDecode,
    
    multiversx_sc::derive::TypeAbi,
    
    Clone
)]


pub struct Offer<M: ManagedTypeApi> {

    pub offer_id: u64,
    

    pub creator: ManagedAddress<M>,
    

    pub recipient: ManagedAddress<M>,

    pub amount: BigUint<M>,
    

    pub status: OfferStatus,
    

    pub created_timestamp: u64
}


#[multiversx_sc::contract]

pub trait EscrowContract {
 
    #[init]
    fn init(&self) {
        // last_offer_id storage mapper'ına ilk değer atıyoruz
        // set_if_empty() kullanıyoruz çünkü:
        // - Eğer değer zaten varsa üzerine yazmaz
        // - Thread-safe bir şekilde çalışır
        // - Storage'a güvenli erişim sağlar
        // 0u64 kullanıyoruz çünkü:
        // - Teklifler 1'den başlayacak
        // - u64 tipi negatif değer alamaz
        // - 8 byte ile çok büyük pozitif sayıları tutabilir
        self.last_offer_id().set_if_empty(0u64);
    }

    // create fonksiyonu yeni bir escrow teklifi oluşturur
    // #[payable("EGLD")] makrosu:
    // - Bu fonksiyonun EGLD token'ı kabul edebileceğini belirtir
    // - Otomatik ödeme kontrolü sağlar
    // - Sadece EGLD ödemelerine izin verir
    // #[endpoint] makrosu:
    // - Fonksiyonu dışarıdan çağrılabilir yapar
    // - ABI'de görünmesini sağlar
    // - Gas limitlerini ayarlar
    #[payable("EGLD")]
    #[endpoint]
    fn create(&self, buyer: ManagedAddress) {
        // Gönderilen EGLD miktarını alıyoruz
        // call_value() kullanıyoruz çünkü:
        // - Güvenli bir şekilde ödeme miktarını alır
        // - Ödeme tipini kontrol eder
        // - Gas optimizasyonu sağlar
        let payment = self.call_value().egld_value();
        
        // Ödeme miktarının 0'dan büyük olduğunu kontrol ediyoruz
        // require! makrosu kullanıyoruz çünkü:
        // - Koşul sağlanmazsa işlemi otomatik olarak geri alır (revert)
        // - Hata mesajı döndürür
        // - Gas'i optimize eder
        require!(payment.clone_value() > BigUint::zero(), "Must pay more than 0");
    
        // İşlemi başlatan adresi (satıcı) alıyoruz
        // blockchain().get_caller() kullanıyoruz çünkü:
        // - Güvenli bir şekilde çağıran adresi alır
        // - Spoofing'e karşı koruma sağlar
        // - Blockchain context'ine güvenli erişim sağlar
        let seller = self.blockchain().get_caller();
        
        // Yeni teklif ID'si oluşturuyoruz
        // Mevcut son ID'yi alıp 1 artırıyoruz çünkü:
        // - Benzersiz ID'ler üretmemiz gerekiyor
        // - Sıralı ID'ler takibi kolaylaştırır
        let new_offer_id = self.last_offer_id().get() + 1;
        
        // Son teklif ID'sini güncelliyoruz
        // set() kullanıyoruz çünkü:
        // - Değeri güvenli şekilde depolar
        // - Storage'a atomik yazma sağlar
        self.last_offer_id().set(new_offer_id);
    
        // Yeni teklif nesnesi oluşturuyoruz
        // Offer struct'ını kullanıyoruz çünkü:
        // - Tüm teklif verilerini organize tutar
        // - Type-safety sağlar
        // - Veri bütünlüğünü korur
        let offer = Offer {
            offer_id: new_offer_id,
            creator: seller.clone(), // clone() ile güvenli kopya alıyoruz
            recipient: buyer.clone(),
            amount: payment.clone_value(),
            status: OfferStatus::Active, // Başlangıç durumu Active
            created_timestamp: self.blockchain().get_block_timestamp() // Zaman damgası
        };
    
        // Teklifi blockchain'e kaydediyoruz
        // offer() storage mapper'ı kullanıyoruz çünkü:
        // - Key-value şeklinde güvenli depolama sağlar
        // - Gas açısından optimize edilmiştir
        // - Serialization/deserialization otomatiktir
        self.offer(new_offer_id).set(offer.clone());
        
        // Satıcının teklifler listesine ekliyoruz
        // UnorderedSet kullanıyoruz çünkü:
        // - Tekrar eden değerleri önler
        // - Hızlı arama sağlar
        // - Gas açısından verimlidir
        self.user_offers(&seller).insert(new_offer_id);
        
        // Alıcının gelen teklifler listesine ekliyoruz
        // Ayrı bir liste tutuyoruz çünkü:
        // - Alıcı kendi gelen tekliflerini kolayca görebilir
        // - İndeksleme ve filtreleme kolaylaşır
        self.user_incoming_offers(&buyer).insert(new_offer_id);
    
        // Teklif oluşturma olayını yayınlıyoruz
        // Event kullanıyoruz çünkü:
        // - Dış sistemler işlemi takip edebilir
        // - Frontend uygulamalar güncellenebilir
        // - İşlem geçmişi blockchain'de loglanır
        self.create_offer_event(new_offer_id, &seller, &buyer, &payment);
    }
    #[endpoint(cancelOffer)]
    fn cancel_offer(
        // &self parametresi, Rust'ta nesne yönelimli programlamada instance metodlarını belirtir
        // Bu sayede kontrat storage'ına erişim sağlanır
        &self,
        // offer_id parametresi, iptal edilecek teklifin benzersiz kimlik numarasıdır
        // u64 tipi kullanılır çünkü negatif ID olamaz ve 64-bit yeterli büyüklüktedir
        offer_id: u64
    ) -> SCResult<()> { // SCResult dönüş tipi, akıllı kontrat işlemlerinde hata yönetimi için kullanılır
        
        // blockchain().get_caller() fonksiyonu çağrıyı yapan adresi güvenli şekilde alır
        // Bu kritik bir güvenlik kontrolüdür - her zaman gerçek çağıranı doğru tespit etmeliyiz
        let caller = self.blockchain().get_caller();
        
        // offer() storage mapper'ı ile blockchain'den teklif bilgilerini okuyoruz
        // mut keyword'ü değişkeni değiştirilebilir yapar çünkü sonra status'ü değiştireceğiz
        let mut offer = self.offer(offer_id).get();

        // require! makrosu ile kritik iş mantığı kontrollerini yapıyoruz
        // Bu kontroller başarısız olursa işlem geri alınır ve EGLD iade edilir
        // Aktif olmayan teklif iptal edilemez - bu mantıksal bir gerekliliktir
        require!(offer.status == OfferStatus::Active, "Offer not active");
        
        // Sadece teklifi oluşturan kişi iptal edebilir
        // Bu önemli bir güvenlik kontrolüdür - başkasının teklifini iptal edemezsiniz
        require!(offer.creator == caller, "Not offer creator");

        // Teklifin durumunu Cancelled olarak güncelliyoruz
        // Enum kullanımı type-safety sağlar, geçersiz durumlar oluşamaz
        offer.status = OfferStatus::Cancelled;
        
        // Güncellenmiş teklifi blockchain'e kaydediyoruz
        // set() fonksiyonu storage'ı güvenli şekilde günceller
        self.offer(offer_id).set(&offer);

        // send().direct_egld() ile kilitli EGLD'yi teklif sahibine geri gönderiyoruz
        // Bu transfer güvenli ve atomiktir - ya tamamen başarılı olur ya da işlem geri alınır
        self.send().direct_egld(&caller, &offer.amount);
        
        // İptal işlemini blockchain'de logluyoruz
        // Bu sayede frontend uygulamalar ve kullanıcılar işlemi takip edebilir
        self.cancel_offer_event(offer_id, &caller, &offer.amount);

        // İşlem başarılı - Ok(()) ile başarı durumunu dönüyoruz
        // () unit type, değer döndürmediğimizi ama işlemin başarılı olduğunu belirtir
        SCResult::Ok(())
    }

    // Teklifi kabul etme fonksiyonu - Bu endpoint sayesinde alıcı kendisine gelen teklifi kabul edebilir
    // #[endpoint(acceptOffer)] attribute'u bu fonksiyonun blockchain üzerinden çağrılabilir olduğunu belirtir
    // acceptOffer ismi, fonksiyonun blockchain üzerinden çağrılırken kullanılacak public ismidir
    #[endpoint(acceptOffer)]
    fn accept_offer(
        // &self parametresi, Rust'ta nesne yönelimli programlamada instance metodlarını belirtir
        // Bu sayede kontrat storage'ına güvenli erişim sağlanır
        &self,
        // offer_id: Kabul edilecek teklifin benzersiz kimlik numarası
        // u64 tipi kullanılır çünkü negatif ID olamaz ve 64-bit yeterli büyüklüktedir
        offer_id: u64
    ) -> SCResult<()> { // SCResult dönüş tipi, akıllı kontrat işlemlerinde hata yönetimi için kullanılır
        
        // blockchain().get_caller() fonksiyonu çağrıyı yapan adresi güvenli şekilde alır
        // Bu kritik bir güvenlik kontrolüdür - her zaman gerçek çağıranı doğru tespit etmeliyiz
        // caller değişkeni immutable olarak tanımlanır çünkü değiştirilmemesi gerekir
        let caller = self.blockchain().get_caller();

        // offer() storage mapper'ı ile blockchain'den teklif bilgilerini okuyoruz
        // mut keyword'ü değişkeni değiştirilebilir yapar çünkü sonra status'ü değiştireceğiz
        // get() fonksiyonu storage'dan veriyi okur ve deserialize eder
        let mut offer = self.offer(offer_id).get();

        // require! makrosu ile kritik iş mantığı kontrollerini yapıyoruz
        // Bu kontroller başarısız olursa işlem geri alınır ve EGLD iade edilir
        // Aktif olmayan teklif kabul edilemez - bu mantıksal bir gerekliliktir
        require!(offer.status == OfferStatus::Active, "Offer not active");

        // Sadece teklifteki alıcı (recipient) kabul edebilir
        // Bu önemli bir güvenlik kontrolüdür - başkasının teklifini kabul edemezsiniz
        require!(offer.recipient == caller, "Not offer recipient");

        // Teklifin durumunu Completed olarak güncelliyoruz
        // Enum kullanımı type-safety sağlar, geçersiz durumlar oluşamaz
        // Bu güncelleme teklifin tamamlandığını belirtir
        offer.status = OfferStatus::Completed;

        // Güncellenmiş teklifi blockchain'e kaydediyoruz
        // set() fonksiyonu storage'ı güvenli şekilde günceller
        // Bu kritik bir adımdır - durumu kalıcı olarak değiştirir
        self.offer(offer_id).set(&offer);

        // send().direct_egld() ile kilitli EGLD'yi alıcıya gönderiyoruz
        // Bu transfer güvenli ve atomiktir - ya tamamen başarılı olur ya da işlem geri alınır
        // Alıcı adresine ve orijinal teklif miktarına göre transfer yapılır
        self.send().direct_egld(&caller, &offer.amount);

        // Kabul işlemini blockchain'de logluyoruz
        // Bu sayede frontend uygulamalar ve kullanıcılar işlemi takip edebilir
        // Event parametreleri: teklif ID'si, kabul eden adres ve miktar
        self.accept_offer_event(offer_id, &caller, &offer.amount);

        // İşlem başarılı - Ok(()) ile başarı durumunu dönüyoruz
        // () unit type, değer döndürmediğimizi ama işlemin başarılı olduğunu belirtir
        SCResult::Ok(())
    }

    // Storage Mappers - Blockchain'de veri depolama yapıları

    // Son teklif ID'sini tutan mapper
    // #[view] attribute'u bu fonksiyonun dışarıdan okunabilir olduğunu belirtir
    // getLastOfferId ismi ile dışarıdan çağrılabilir
    // Bu sayede frontend uygulamalar son teklif ID'sini sorgulayabilir
    #[view(getLastOfferId)]
    // #[storage_mapper] attribute'u bu değişkenin blockchain storage'ında saklanacağını belirtir
    // "lastOfferId" string'i storage'da bu veriyi tanımlayan benzersiz bir key olarak kullanılır
    #[storage_mapper("lastOfferId")] 
    // SingleValueMapper<u64> tipi kullanılır çünkü:
    // - Tek bir değer saklanacak (son ID)
    // - u64 tipi negatif olmayan ve yeterince büyük sayılar için uygundur
    // - SingleValueMapper otomatik serialize/deserialize işlemlerini halleder
    fn last_offer_id(&self) -> SingleValueMapper<u64>;

    // Teklif bilgilerini ID'ye göre tutan mapper
    // #[view] ile dışarıdan okunabilir yapılır
    // getOffer ismi ile frontend'den çağrılabilir
    #[view(getOffer)]
    // "offer" string'i storage key olarak kullanılır
    // Her teklif için ID'ye göre ayrı bir storage alanı oluşturulur
    #[storage_mapper("offer")]
    // id parametresi ile hangi teklifin bilgilerinin istediği belirtilir
    // Offer<Self::Api> tipi teklif verilerinin yapısını tanımlar
    // SingleValueMapper serialize/deserialize işlemlerini otomatik yapar
    fn offer(&self, id: u64) -> SingleValueMapper<Offer<Self::Api>>;

    // Kullanıcının oluşturduğu tekliflerin ID'lerini tutan mapper
    // #[view] ile dışarıdan okunabilir
    // getUserOffers ismi ile frontend'den çağrılabilir
    #[view(getUserOffers)]
    // "userOffers" string'i storage key olarak kullanılır
    // Her kullanıcı için ayrı bir set oluşturulur
    #[storage_mapper("userOffers")]
    // SetMapper kullanılır çünkü:
    // - Bir kullanıcının birden fazla teklifi olabilir
    // - Set yapısı tekrar eden değerleri engeller
    // - Verimli arama/ekleme/silme operasyonları sağlar
    fn user_offers(&self, user: &ManagedAddress) -> SetMapper<u64>;

    // Kullanıcıya gelen tekliflerin ID'lerini tutan mapper
    // #[view] ile dışarıdan okunabilir
    // getUserIncomingOffers ismi ile frontend'den çağrılabilir
    #[view(getUserIncomingOffers)]
    // "userIncomingOffers" string'i storage key olarak kullanılır
    // Her alıcı için ayrı bir set oluşturulur
    #[storage_mapper("userIncomingOffers")]
    // SetMapper aynı sebeplerden dolayı kullanılır
    // ManagedAddress referansı ile bellek optimizasyonu sağlanır
    fn user_incoming_offers(&self, user: &ManagedAddress) -> SetMapper<u64>;

    // Events - Blockchain'de kaydedilen olaylar
    // Eventler blockchain'de kalıcı olarak loglanır
    // Frontend uygulamalar bu eventleri dinleyerek değişiklikleri takip edebilir

    // Teklif oluşturma olayı
    // createOffer isimli event blockchain'e kaydedilir
    #[event("createOffer")]
    // Event parametreleri #[indexed] ile işaretlenir
    // Bu sayede parametrelere göre filtreleme/arama yapılabilir
    fn create_offer_event(
        &self,
        #[indexed] offer_id: u64,        // Teklifin benzersiz ID'si
        #[indexed] creator: &ManagedAddress,  // Teklifi oluşturan adres
        #[indexed] recipient: &ManagedAddress, // Alıcı adresi
        #[indexed] amount: &BigUint       // Teklif miktarı (EGLD)
    );

    // Teklif iptal olayı
    // cancelOffer isimli event blockchain'e kaydedilir
    #[event("cancelOffer")]
    fn cancel_offer_event(
        &self,
        #[indexed] offer_id: u64,        // İptal edilen teklifin ID'si
        #[indexed] creator: &ManagedAddress,  // Teklifi iptal eden (oluşturan) adres
        #[indexed] amount: &BigUint       // İade edilen miktar
    );

    // Teklif kabul olayı
    // acceptOffer isimli event blockchain'e kaydedilir
    #[event("acceptOffer")]
    fn accept_offer_event(
        &self,
        #[indexed] offer_id: u64,        // Kabul edilen teklifin ID'si
        #[indexed] recipient: &ManagedAddress, // Teklifi kabul eden (alıcı) adres
        #[indexed] amount: &BigUint       // Transfer edilen miktar
    );
    // View fonksiyonları - Blockchain'den sadece veri okuma işlemleri yapan fonksiyonlardır
    // Bu fonksiyonlar blockchain'i değiştirmez, sadece mevcut durumu sorgular
    // Gas maliyeti düşüktür çünkü durum değişikliği yapmazlar

    // Tüm aktif teklifleri getiren fonksiyon
    // #[view] attribute'u bu fonksiyonun dışarıdan okunabilir olduğunu belirtir
    // getActiveOffers ismi ile frontend'den çağrılabilir
    #[view(getActiveOffers)]
    // &self parametresi kontrat instance'ına erişim sağlar
    // -> MultiValueEncoded<Offer<Self::Api>> dönüş tipi birden fazla Offer'ı encode edilmiş formatta döndürür
    // Bu format blockchain üzerinden veri transferi için optimize edilmiştir
    fn get_active_offers(&self) -> MultiValueEncoded<Offer<Self::Api>> {
        // MultiValueEncoded.new() ile boş bir sonuç listesi oluşturulur
        // mut keyword'ü ile değiştirilebilir olduğu belirtilir
        // Rust'ta varsayılan olarak değişkenler immutable'dır, güvenlik için
        let mut result = MultiValueEncoded::new();
        
        // 1'den son teklif ID'sine kadar olan tüm ID'leri kontrol et
        // ..= operatörü son değeri de dahil eder (inclusive range)
        // last_offer_id().get() ile storage'dan son ID değeri okunur
        for offer_id in 1..=self.last_offer_id().get() {
            // offer() storage mapper'ı ile ID'ye karşılık gelen teklif bilgileri alınır
            // get() ile storage'dan veri okunur
            let offer = self.offer(offer_id).get();
            
            // Sadece durumu Active olan teklifler listeye eklenir
            // == operatörü ile enum değerleri karşılaştırılır
            if offer.status == OfferStatus::Active {
                // push() ile listeye yeni eleman eklenir
                result.push(offer);
            }
        }
        
        // Fonksiyon sonunda result otomatik olarak return edilir
        // Rust'ta son ifadeden sonra ; konmazsa return anlamına gelir
        result
    }

    // Belirli bir kullanıcının aktif tekliflerini getiren fonksiyon
    // getUserActiveOffers ismi ile frontend'den çağrılabilir
    // #[view] attribute'u ile bu fonksiyonun dışarıdan okunabilir olduğunu belirtiyoruz
    // getUserActiveOffers ismi ile frontend tarafından çağrılabilir hale getiriyoruz
    // View fonksiyonları blockchain durumunu değiştirmez, sadece veri okur
    // Bu sayede gas maliyeti düşük olur ve hızlı çalışır
    #[view(getUserActiveOffers)]
    // get_user_active_offers fonksiyonu belirli bir kullanıcının aktif tekliflerini getirir
    // &self parametresi kontrat instance'ına erişim sağlar, storage'a ulaşmak için gerekli
    // user parametresi ManagedAddress türünde blockchain adresi alır
    // & ile referans alarak gereksiz kopya oluşturmayı engelleriz, bellek optimizasyonu sağlarız
    // -> MultiValueEncoded<Offer> dönüş tipi birden fazla teklifi encode edilmiş formatta döndürür
    // Bu format blockchain üzerinden veri transferi için optimize edilmiştir
    fn get_user_active_offers(
        &self,
        user: &ManagedAddress
    ) -> MultiValueEncoded<Offer<Self::Api>> {
        // MultiValueEncoded.new() ile boş bir sonuç listesi oluşturuyoruz
        // mut keyword'ü ile değiştirilebilir olduğunu belirtiyoruz
        // Rust'ta varsayılan olarak değişkenler immutable'dır
        // Bu güvenlik için önemlidir, değişkenlerin yanlışlıkla değiştirilmesini engeller
        let mut result = MultiValueEncoded::new();
        
        // user_offers() storage mapper'ı kullanıcının tüm teklif ID'lerini tutar
        // iter() metodu ile bu ID'ler üzerinde döngü kuruyoruz
        // Bu şekilde kullanıcının tüm tekliflerini kontrol edebiliyoruz
        // Storage mapper'lar blockchain storage'ına erişim sağlar
        for offer_id in self.user_offers(user).iter() {
            // offer() storage mapper'ı ile her ID'ye karşılık gelen teklif bilgilerini okuyoruz
            // get() metodu storage'dan veriyi çeker
            // Bu işlem her döngüde bir storage okuma maliyeti oluşturur
            let offer = self.offer(offer_id).get();
            
            // Teklif durumunu kontrol ediyoruz
            // == operatörü ile enum değerlerini karşılaştırıyoruz
            // Sadece Active durumundaki teklifleri listeye ekliyoruz
            // Bu filtreleme ile iptal edilmiş veya tamamlanmış teklifleri eliyoruz
            if offer.status == OfferStatus::Active {
                // push() metodu ile aktif teklifi sonuç listesine ekliyoruz
                // Bu işlem bellek üzerinde gerçekleşir, storage'a yazma yapılmaz
                result.push(offer);
            }
        }
        
        // Rust'ta son satırda ; olmadığında o değer return edilir
        // Bu şekilde return keyword'ü kullanmadan sonucu döndürmüş oluyoruz
        // Fonksiyon bitiminde result değişkeni scope'dan çıkarak bellekten temizlenir
        // RAII (Resource Acquisition Is Initialization) prensibi gereği
        // Rust otomatik olarak scope dışına çıkan değişkenleri temizler
        result
    }
    // Belirli bir kullanıcıya gelen aktif teklifleri getiren fonksiyon
    // Bu fonksiyon view (sadece okuma) tipinde olup, blockchain durumunu değiştirmez
    // Frontend'den getUserIncomingActiveOffers ismiyle çağrılabilir
    #[view(getUserIncomingActiveOffers)]
    fn get_user_incoming_active_offers(
        // &self parametresi ile kontrat instance'ına erişim sağlanır
        // Bu sayede storage'daki verilere ulaşabiliriz
        &self,
        // user parametresi blockchain adresini referans olarak alır
        // ManagedAddress türü bellek optimizasyonu için özel bir tür
        // & ile referans alarak gereksiz kopya oluşturmayı engelleriz
        user: &ManagedAddress
    ) -> MultiValueEncoded<Offer<Self::Api>> {
        // MultiValueEncoded türünde bir sonuç listesi oluşturuyoruz
        // mut keyword'ü ile değiştirilebilir olduğunu belirtiyoruz
        // new() ile boş bir liste başlatıyoruz
        // Bu liste blockchain'e uygun formatta kodlanmış verileri tutar
        let mut result = MultiValueEncoded::new();
        
        // user_incoming_offers storage mapper'ı ile kullanıcıya gelen tekliflerin ID'lerini alıyoruz
        // iter() metodu ile bu ID'ler üzerinde döngü kuruyoruz
        // Bu şekilde tüm gelen teklifleri kontrol edebiliyoruz
        for offer_id in self.user_incoming_offers(user).iter() {
            // offer() storage mapper'ı ile her ID'ye karşılık gelen teklif bilgilerini okuyoruz
            // get() metodu storage'dan veriyi çeker
            // Bu işlem her döngüde bir storage okuma maliyeti oluşturur
            let offer = self.offer(offer_id).get();

            // Teklif durumunu kontrol ediyoruz
            // == operatörü ile enum değerlerini karşılaştırıyoruz
            // Sadece Active durumundaki teklifleri listeye ekliyoruz
            // Bu filtreleme ile iptal edilmiş veya tamamlanmış teklifleri elemış oluyoruz
            if offer.status == OfferStatus::Active {
                // push() metodu ile aktif teklifi sonuç listesine ekliyoruz
                // Bu işlem bellek üzerinde gerçekleşir, storage'a yazma yapılmaz
                result.push(offer);
            }
        }
        



        result
    }
}